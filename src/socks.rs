//! Servidor SOCKS5 local. É o único endereço que o Discord conhece.
//!
//! Para cada conexão ele consulta `routing` e decide: encaminhar por um proxy
//! estrangeiro da piscina, ou abrir direto. O Discord não sabe a diferença.

use crate::{
    pool::Pool,
    routing::{self, Modo, Rota},
    sessao::Sessao,
};
use anyhow::{bail, Result};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::broadcast,
    time::{timeout, Duration},
};

const CONEXAO_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn servir(porta: u16, pool: Pool, modo: Modo, sessao: Arc<Sessao>) -> Result<()> {
    let escuta = TcpListener::bind(("127.0.0.1", porta)).await?;
    log::linha(&format!("proxy local escutando em 127.0.0.1:{porta}"));

    loop {
        let (cliente, _) = escuta.accept().await?;
        let pool = pool.clone();
        let sessao = sessao.clone();
        tokio::spawn(async move {
            let _ = cliente.set_nodelay(true);
            if let Err(e) = atender(cliente, pool, modo, sessao).await {
                log::linha(&format!("conexão encerrada: {e}"));
            }
        });
    }
}

async fn atender(
    mut cliente: TcpStream,
    pool: Pool,
    modo: Modo,
    sessao: Arc<Sessao>,
) -> Result<()> {
    let (host, porta) = aperto_de_mao(&mut cliente).await?;

    // Assinar antes de ler a fase fecha a corrida: se a janela fechar daqui em
    // diante, esta conexão recebe o aviso e cai junto. Na ordem inversa ela
    // nasceria no exterior logo depois do aviso e ficaria presa lá.
    let aviso = sessao.assinar_cancelamento();
    let rota = routing::decidir(&host, modo, sessao.fase());

    let mut cancelar = None;
    let servidor = match rota {
        Rota::Exterior => {
            sessao.registrar_controle(Instant::now());
            match abrir_pelo_exterior(&pool, &host, porta).await {
                Ok(s) => {
                    log::linha(&format!("exterior  {host}:{porta}"));
                    cancelar = Some(aviso);
                    s
                }
                Err(e) => {
                    // Sem upstream sadio é melhor entregar direto do que derrubar o
                    // Discord: perde-se a correção, não a conexão.
                    log::linha(&format!("exterior indisponível ({e}); {host} vai direto"));
                    abrir_direto(&host, porta).await?
                }
            }
        }
        Rota::Direta => {
            log::linha(&format!("direto    {host}:{porta}"));
            abrir_direto(&host, porta).await?
        }
    };

    responder_ok(&mut cliente).await?;
    encaminhar(cliente, servidor, cancelar).await
}

/// Lê a saudação e o pedido CONNECT do cliente. Devolve destino e porta.
async fn aperto_de_mao(cliente: &mut TcpStream) -> Result<(String, u16)> {
    let mut cab = [0u8; 2];
    cliente.read_exact(&mut cab).await?;
    if cab[0] != 0x05 {
        bail!("versão SOCKS não suportada: {}", cab[0]);
    }
    let mut metodos = vec![0u8; cab[1] as usize];
    cliente.read_exact(&mut metodos).await?;
    cliente.write_all(&[0x05, 0x00]).await?; // sem autenticação

    let mut req = [0u8; 4];
    cliente.read_exact(&mut req).await?;
    if req[1] != 0x01 {
        recusar(cliente, 0x07).await?;
        bail!("só CONNECT é suportado");
    }

    let host = match req[3] {
        0x01 => {
            let mut o = [0u8; 4];
            cliente.read_exact(&mut o).await?;
            format!("{}.{}.{}.{}", o[0], o[1], o[2], o[3])
        }
        0x03 => {
            let mut n = [0u8; 1];
            cliente.read_exact(&mut n).await?;
            let mut d = vec![0u8; n[0] as usize];
            cliente.read_exact(&mut d).await?;
            String::from_utf8(d)?
        }
        0x04 => {
            let mut o = [0u8; 16];
            cliente.read_exact(&mut o).await?;
            std::net::Ipv6Addr::from(o).to_string()
        }
        outro => {
            recusar(cliente, 0x08).await?;
            bail!("tipo de endereço desconhecido: {outro}");
        }
    };

    let mut p = [0u8; 2];
    cliente.read_exact(&mut p).await?;
    Ok((host, u16::from_be_bytes(p)))
}

async fn abrir_direto(host: &str, porta: u16) -> Result<TcpStream> {
    let s = timeout(CONEXAO_TIMEOUT, TcpStream::connect((host, porta))).await??;
    let _ = s.set_nodelay(true);
    Ok(s)
}

async fn abrir_pelo_exterior(pool: &Pool, host: &str, porta: u16) -> Result<TcpStream> {
    // Duas tentativas: se o melhor upstream morreu, ele é rebaixado e a próxima
    // volta já pega outro.
    for _ in 0..2 {
        let Some(upstream) = pool.melhor() else {
            bail!("piscina vazia");
        };
        match encadear(&upstream, host, porta).await {
            Ok(s) => return Ok(s),
            Err(e) => {
                log::linha(&format!("upstream {upstream} falhou: {e}"));
                pool.marcar_falha(&upstream);
            }
        }
    }
    bail!("nenhum upstream respondeu")
}

/// Faz o aperto de mão SOCKS5 contra o proxy estrangeiro, pedindo `host:porta`.
async fn encadear(upstream: &str, host: &str, porta: u16) -> Result<TcpStream> {
    let endereco: SocketAddr = upstream.parse()?;
    let mut s = timeout(CONEXAO_TIMEOUT, TcpStream::connect(endereco)).await??;
    let _ = s.set_nodelay(true);

    s.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut resp = [0u8; 2];
    s.read_exact(&mut resp).await?;
    if resp != [0x05, 0x00] {
        bail!("upstream recusou o método sem autenticação");
    }

    let h = host.as_bytes();
    if h.len() > 255 {
        bail!("host longo demais");
    }
    let mut pedido = vec![0x05, 0x01, 0x00, 0x03, h.len() as u8];
    pedido.extend_from_slice(h);
    pedido.extend_from_slice(&porta.to_be_bytes());
    s.write_all(&pedido).await?;

    let mut cab = [0u8; 4];
    s.read_exact(&mut cab).await?;
    if cab[1] != 0x00 {
        bail!("upstream recusou a conexão (código {})", cab[1]);
    }
    // Consome o endereço de ligação, que não usamos.
    match cab[3] {
        0x01 => {
            let mut d = [0u8; 4];
            s.read_exact(&mut d).await?;
        }
        0x03 => {
            let mut n = [0u8; 1];
            s.read_exact(&mut n).await?;
            let mut d = vec![0u8; n[0] as usize];
            s.read_exact(&mut d).await?;
        }
        0x04 => {
            let mut d = [0u8; 16];
            s.read_exact(&mut d).await?;
        }
        outro => bail!("resposta do upstream ilegível: atyp {outro}"),
    }
    let mut p = [0u8; 2];
    s.read_exact(&mut p).await?;
    Ok(s)
}

async fn responder_ok(cliente: &mut TcpStream) -> Result<()> {
    cliente
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;
    Ok(())
}

async fn recusar(cliente: &mut TcpStream, codigo: u8) -> Result<()> {
    cliente
        .write_all(&[0x05, codigo, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;
    Ok(())
}

/// Bombeia os dois lados até um deles fechar — ou até a janela de abertura
/// fechar, quando `cancelar` está presente.
///
/// Só as conexões que saíram pelo exterior recebem a assinatura. Elas são as
/// únicas que precisam morrer: o websocket do gateway vive horas, e sem esse
/// empurrão ele ficaria preso no proxy estrangeiro pelo resto da sessão,
/// carregando toda mensagem por um caminho que já não corrige nada. Derrubar
/// aqui faz o Discord reconectar direto com RESUME — a mesma coisa que
/// desligar a VPN depois de abrir o programa, que é a correção manual que este
/// projeto automatiza.
async fn encaminhar(
    mut a: TcpStream,
    mut b: TcpStream,
    cancelar: Option<broadcast::Receiver<()>>,
) -> Result<()> {
    let Some(mut cancelar) = cancelar else {
        tokio::io::copy_bidirectional(&mut a, &mut b).await?;
        return Ok(());
    };

    tokio::select! {
        r = tokio::io::copy_bidirectional(&mut a, &mut b) => { r?; }
        _ = cancelar.recv() => {
            let _ = a.shutdown().await;
            let _ = b.shutdown().await;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Duas pontas ligadas de verdade, para exercitar o `encaminhar` como ele
    /// roda em produção — sem simulacro de socket.
    async fn par() -> (TcpStream, TcpStream) {
        let escuta = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let endereco = escuta.local_addr().unwrap();
        let cliente = TcpStream::connect(endereco).await.unwrap();
        let (servidor, _) = escuta.accept().await.unwrap();
        (cliente, servidor)
    }

    #[tokio::test]
    async fn fechar_a_janela_derruba_quem_saiu_pelo_exterior() {
        let (a, _guarda_a) = par().await;
        let (b, _guarda_b) = par().await;

        let (avisar, escutar) = broadcast::channel(1);
        let bombeando = tokio::spawn(encaminhar(a, b, Some(escutar)));

        // Ninguém mandou nada e ninguém fechou: sem o aviso isto ficaria
        // parado para sempre, que é o gateway preso no proxy estrangeiro.
        avisar.send(()).unwrap();

        let fim = timeout(Duration::from_secs(2), bombeando).await;
        assert!(
            fim.is_ok(),
            "a conexão devia ter caído assim que a janela fechou"
        );
        assert!(fim.unwrap().unwrap().is_ok());
    }

    #[tokio::test]
    async fn conexao_direta_nao_e_derrubada_pela_janela() {
        let (a, _guarda_a) = par().await;
        let (b, _guarda_b) = par().await;

        // Sem assinatura: é o caso de quem já ia direto e não deve nada à
        // janela de abertura.
        let bombeando = tokio::spawn(encaminhar(a, b, None));

        let fim = timeout(Duration::from_millis(300), bombeando).await;
        assert!(fim.is_err(), "conexão direta continua de pé");
    }
}

pub mod log {
    use std::{
        io::Write,
        sync::{Mutex, OnceLock},
    };

    const TAMANHO_MAXIMO: u64 = 512 * 1024;

    /// Escritas são serializadas: sem isso, threads concorrentes intercalam
    /// linhas no meio umas das outras.
    fn tranca() -> &'static Mutex<()> {
        static T: OnceLock<Mutex<()>> = OnceLock::new();
        T.get_or_init(|| Mutex::new(()))
    }

    pub fn linha(msg: &str) {
        let _guarda = tranca().lock();
        let caminho = crate::caminho_log();

        // O log é para diagnóstico, não para histórico: passou do teto, recomeça.
        if std::fs::metadata(&caminho).map(|m| m.len()).unwrap_or(0) > TAMANHO_MAXIMO {
            let _ = std::fs::write(&caminho, b"");
        }

        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&caminho)
        {
            let _ = writeln!(f, "{msg}");
        }
        println!("{msg}");
    }
}

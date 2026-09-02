//! Servidor SOCKS5 local. É o único endereço que o Discord conhece.
//!
//! Para cada conexão ele consulta `routing` e decide: encaminhar por um proxy
//! estrangeiro da piscina, ou abrir direto. O Discord não sabe a diferença.

use crate::{
    pool::Pool,
    routing::{self, Rota},
    sessao::Sessao,
};
use anyhow::{bail, Result};
use std::{net::SocketAddr, sync::Arc, time::Instant};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, error::TryRecvError},
    time::{timeout, Duration},
};

/// Quanto esperar por uma conexão direta com o Discord.
const DIRETO_TIMEOUT: Duration = Duration::from_secs(15);

/// Quanto esperar por um proxy estrangeiro — o TCP e o aperto de mão SOCKS5
/// juntos. Curto de propósito: durante a janela cada requisição do Discord
/// paga esta espera quando o upstream está morto, e um proxy que a piscina
/// validou com uma requisição HTTPS inteira em menos de oito segundos não tem
/// desculpa para demorar mais do que isto num aperto de mão.
///
/// Antes eram 15 s só para o TCP, e o aperto de mão em si não tinha prazo
/// nenhum: um upstream que aceitava a conexão e não respondia prendia a
/// tarefa para sempre — e, com ela, a contagem do silêncio.
const EXTERIOR_TIMEOUT: Duration = Duration::from_secs(5);

/// Quantos upstreams tentar antes de desistir do exterior. Dois: se o melhor
/// morreu, a segunda volta pula para o próximo da fila, e a abertura continua
/// saindo por fora em vez de cair direto na primeira falha. O pior caso é o
/// dobro do prazo acima.
const TENTATIVAS_EXTERIOR: usize = 2;

pub async fn servir(porta: u16, pool: Pool, sessao: Arc<Sessao>) -> Result<()> {
    let escuta = TcpListener::bind(("127.0.0.1", porta)).await?;
    log::linha(&format!("proxy local escutando em 127.0.0.1:{porta}"));

    loop {
        let (cliente, _) = escuta.accept().await?;
        let pool = pool.clone();
        let sessao = sessao.clone();
        tokio::spawn(async move {
            let _ = cliente.set_nodelay(true);
            if let Err(e) = atender(cliente, pool, sessao).await {
                log::linha(&format!("conexão encerrada: {e}"));
            }
        });
    }
}

async fn atender(mut cliente: TcpStream, pool: Pool, sessao: Arc<Sessao>) -> Result<()> {
    let (host, porta) = aperto_de_mao(&mut cliente).await?;

    // Assinar antes de ler a fase fecha a corrida: se a janela fechar daqui em
    // diante, esta conexão recebe o aviso e cai junto. Na ordem inversa ela
    // nasceria no exterior logo depois do aviso e ficaria presa lá.
    let mut aviso = sessao.assinar_cancelamento();
    let rota = routing::decidir(&host, sessao.fase());

    let (servidor, cancelar) = match rota {
        Rota::Exterior => {
            let aberta = {
                // Enquanto o aperto de mão com o upstream não termina, a janela
                // não pode vencer — mas só para as conexões que decidem a
                // região. `comecar_aperto` devolve `None` para as outras, e
                // elas não seguram nada.
                let _aperto = sessao.comecar_aperto(&host, Instant::now());
                abrir_pelo_exterior(&pool, &host, porta).await
            };

            match aberta {
                Ok(_) if fechou_no_meio(&mut aviso) => {
                    // A janela fechou enquanto o aperto de mão corria. Esta
                    // conexão nasceria no exterior só para cair no instante
                    // seguinte — e o Discord, que já tinha recebido o OK,
                    // veria o download morrer logo no início. Como o OK ainda
                    // não foi enviado, dá para abrir direto sem ele perceber.
                    log::linha(&format!("a janela fechou durante o aperto de mão; {host} vai direto"));
                    log::linha(&format!("direto    {host}:{porta}"));
                    (abrir_direto(&host, porta).await?, None)
                }
                Ok(s) => {
                    log::linha(&format!("exterior  {host}:{porta}"));
                    (s, Some(aviso))
                }
                Err(e) => {
                    // Sem upstream sadio é melhor entregar direto do que
                    // derrubar o Discord: perde-se a correção, não a conexão.
                    log::linha(&format!("exterior indisponível ({e}); {host} vai direto"));
                    log::linha(&format!("direto    {host}:{porta}"));
                    (abrir_direto(&host, porta).await?, None)
                }
            }
        }
        Rota::Direta => {
            log::linha(&format!("direto    {host}:{porta}"));
            (abrir_direto(&host, porta).await?, None)
        }
    };

    responder_ok(&mut cliente).await?;

    // O gateway é a conexão pela qual a sessão vive. Acompanhar quando ele
    // abre e fecha é o que permite suspeitar de uma sessão que renasceu pelo
    // IP brasileiro depois de a janela fechar — ver `Sessao::gateway_abriu`.
    let _gateway = routing::e_gateway(&host).then(|| GatewayAberto::novo(&sessao));
    encaminhar(cliente, servidor, cancelar).await
}

/// A janela pode ter fechado entre a assinatura e o fim do aperto de mão
/// com o upstream. O aviso, se veio, está na assinatura.
fn fechou_no_meio(aviso: &mut broadcast::Receiver<()>) -> bool {
    !matches!(aviso.try_recv(), Err(TryRecvError::Empty))
}

/// Uma conexão com o gateway sendo encaminhada. Registra a suspeita ao nascer
/// e avisa a sessão ao cair, por qualquer caminho.
struct GatewayAberto<'a> {
    sessao: &'a Sessao,
}

impl<'a> GatewayAberto<'a> {
    fn novo(sessao: &'a Sessao) -> Self {
        if let Some(vao) = sessao.gateway_abriu(Instant::now()) {
            log::linha(&format!(
                "suspeita: gateway novo depois de {} s sem gateway nenhum, com a sessão já aberta; \
                 ela pode ter renascido pelo IP brasileiro — se a tela parar de funcionar, reinicie o Discord",
                vao.as_secs()
            ));
        }
        Self { sessao }
    }
}

impl Drop for GatewayAberto<'_> {
    fn drop(&mut self) {
        self.sessao.gateway_fechou(Instant::now());
    }
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
            let nome = String::from_utf8(d)?;
            // Um nome com byte de controle no meio — `evil.com\0.discord.com`
            // — casaria com o sufixo do Discord aqui e seria resolvido como
            // `evil.com` por um upstream escrito em C. O SOCKS local aceita
            // conexão de qualquer programa da máquina, então este é o lugar
            // de fechar a porta.
            if !nome_de_host_valido(&nome) {
                recusar(cliente, 0x08).await?;
                bail!("nome de host inválido no pedido CONNECT");
            }
            nome
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

/// Só o que um nome DNS pode ter: letras, dígitos, ponto, hífen e o
/// sublinhado que alguns registros usam. É o mesmo alfabeto que `routing`
/// exige antes de casar um sufixo.
fn nome_de_host_valido(nome: &str) -> bool {
    routing::nome_bem_formado(nome)
}

async fn abrir_direto(host: &str, porta: u16) -> Result<TcpStream> {
    let s = timeout(DIRETO_TIMEOUT, TcpStream::connect((host, porta))).await??;
    let _ = s.set_nodelay(true);
    Ok(s)
}

async fn abrir_pelo_exterior(pool: &Pool, host: &str, porta: u16) -> Result<TcpStream> {
    abrir_pelo_exterior_com_prazo(pool, host, porta, EXTERIOR_TIMEOUT).await
}

async fn abrir_pelo_exterior_com_prazo(
    pool: &Pool,
    host: &str,
    porta: u16,
    prazo: Duration,
) -> Result<TcpStream> {
    // Quem já falhou nesta conexão não é tentado de novo: uma falha só ainda
    // não tira o upstream da fila, e sem esta lista a segunda volta pegava o
    // mesmo endereço e gastava outro prazo inteiro nele.
    let mut tentados: Vec<String> = Vec::new();
    for _ in 0..TENTATIVAS_EXTERIOR {
        let Some(upstream) = pool.melhor_exceto(&tentados) else {
            if tentados.is_empty() {
                bail!("piscina vazia");
            }
            bail!("nenhum outro upstream na piscina");
        };
        match encadear(&upstream, host, porta, prazo).await {
            Ok(s) => return Ok(s),
            Err(e) => {
                log::linha(&format!("upstream {upstream} falhou: {e}"));
                pool.marcar_falha(&upstream);
                tentados.push(upstream);
            }
        }
    }
    bail!("nenhum upstream respondeu")
}

/// Faz o aperto de mão SOCKS5 contra o proxy estrangeiro, pedindo
/// `host:porta`. O prazo cobre o TCP e o aperto de mão juntos: um upstream que
/// aceita a conexão e cala não pode prender ninguém.
async fn encadear(upstream: &str, host: &str, porta: u16, prazo: Duration) -> Result<TcpStream> {
    match timeout(prazo, encadear_sem_prazo(upstream, host, porta)).await {
        Ok(resultado) => resultado,
        Err(_) => bail!("não respondeu em {:.1} s", prazo.as_secs_f32()),
    }
}

async fn encadear_sem_prazo(upstream: &str, host: &str, porta: u16) -> Result<TcpStream> {
    let endereco: SocketAddr = upstream.parse()?;
    let mut s = TcpStream::connect(endereco).await?;
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

    #[tokio::test]
    async fn upstream_mudo_nao_prende_a_conexao() {
        // Um upstream que aceita o TCP e nunca responde ao SOCKS5. Antes, o
        // aperto de mão não tinha prazo e isto prendia a tarefa para sempre —
        // com a contagem do silêncio presa junto.
        let escuta = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let endereco = escuta.local_addr().unwrap();
        let mudo = tokio::spawn(async move {
            let (conexao, _) = escuta.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
            drop(conexao);
        });

        let inicio = Instant::now();
        let prazo = Duration::from_millis(300);
        let resultado = encadear(&endereco.to_string(), "discord.com", 443, prazo).await;
        let levou = inicio.elapsed();
        mudo.abort();

        assert!(resultado.is_err(), "um upstream mudo é uma falha, não uma espera");
        assert!(
            levou < Duration::from_secs(3),
            "desistiu em {levou:?}, muito depois do prazo de {prazo:?}"
        );
    }

    /// Um upstream SOCKS5 de mentira que aceita qualquer CONNECT. Só o
    /// bastante para `encadear` terminar feliz.
    async fn upstream_que_aceita() -> SocketAddr {
        let escuta = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let endereco = escuta.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut s, _)) = escuta.accept().await else { break };
                tokio::spawn(async move {
                    let mut saudacao = [0u8; 3];
                    if s.read_exact(&mut saudacao).await.is_err() {
                        return;
                    }
                    let _ = s.write_all(&[0x05, 0x00]).await;
                    let mut cab = [0u8; 5];
                    if s.read_exact(&mut cab).await.is_err() {
                        return;
                    }
                    let mut resto = vec![0u8; cab[4] as usize + 2];
                    if s.read_exact(&mut resto).await.is_err() {
                        return;
                    }
                    let _ = s.write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]).await;
                    // Fica de pé até o outro lado desistir.
                    let mut lixo = [0u8; 1];
                    let _ = s.read(&mut lixo).await;
                });
            }
        });
        endereco
    }

    #[tokio::test]
    async fn a_segunda_tentativa_vai_para_outro_upstream() {
        // O melhor da fila aceita o TCP e cala; o segundo funciona. Antes, a
        // segunda volta insistia no primeiro — uma falha só não o tira da
        // fila — e a conexão caía para direto com a piscina cheia.
        let escuta = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let mudo = escuta.local_addr().unwrap();
        let segurando = tokio::spawn(async move {
            let (conexao, _) = escuta.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
            drop(conexao);
        });
        let bom = upstream_que_aceita().await;
        let pool = Pool::de_teste(&[&mudo.to_string(), &bom.to_string()]);

        let inicio = Instant::now();
        let resultado =
            abrir_pelo_exterior_com_prazo(&pool, "discord.com", 443, Duration::from_millis(300))
                .await;
        segurando.abort();

        assert!(resultado.is_ok(), "o segundo upstream devia ter atendido: {resultado:?}");
        assert!(inicio.elapsed() < Duration::from_secs(3));
        assert_eq!(pool.quantidade(), 2, "uma falha só ainda perdoa o mudo");
        assert_eq!(
            pool.melhor().as_deref(),
            Some(mudo.to_string().as_str()),
            "o mudo continua na frente da fila até a segunda falha"
        );
    }

    #[test]
    fn nome_de_host_com_byte_de_controle_e_recusado() {
        assert!(nome_de_host_valido("discord.com"));
        assert!(nome_de_host_valido("gateway-us-east1-b.discord.gg"));
        assert!(nome_de_host_valido("_sip._tcp.exemplo.com"));
        for h in ["evil.com\0.discord.com", "x\nexterior  google.com", "a b.discord.com", ""] {
            assert!(!nome_de_host_valido(h), "{h:?}");
        }
    }

    #[test]
    fn a_janela_que_fechou_durante_o_aperto_de_mao_e_percebida() {
        let (avisar, mut escutar) = broadcast::channel(1);
        assert!(!fechou_no_meio(&mut escutar), "nada aconteceu ainda");

        avisar.send(()).unwrap();
        assert!(
            fechou_no_meio(&mut escutar),
            "a janela fechou entre a assinatura e o fim do aperto de mão"
        );
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

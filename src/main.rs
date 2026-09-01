//! fol-discord — corrige o problema do Discord no Brasil.
//!
//! O Discord decide a região da sua sessão pelo IP que enxerga na abertura.
//! Em vários provedores brasileiros essa decisão sai errada e a transmissão de
//! tela para de funcionar. Este programa faz só o punhado de conexões que
//! determinam essa decisão sair por um IP estrangeiro. A voz, a tela e o resto
//! da internet continuam saindo direto, com o ping de sempre.

#![windows_subsystem = "windows"]

mod pac;
mod pool;
mod routing;
mod socks;
mod windows;

use anyhow::{Context, Result};
use routing::Modo;
use std::{path::PathBuf, time::Duration};

const PORTA_SOCKS: u16 = 9250;
const PORTA_PAC: u16 = 9251;
const MINIMO_SAUDAVEIS: usize = 3;
const INTERVALO_MANUTENCAO: Duration = Duration::from_secs(300);

fn url_pac() -> String {
    format!("http://127.0.0.1:{PORTA_PAC}/proxy.pac")
}

pub fn pasta_dados() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("FolDiscord")
}

pub fn caminho_log() -> PathBuf {
    pasta_dados().join("fol.log")
}

fn caminho_instalado() -> PathBuf {
    pasta_dados().join("fol-discord.exe")
}

fn main() -> Result<()> {
    anexar_console();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let modo = if args.iter().any(|a| a == "--tudo-discord") {
        Modo::TudoDiscord
    } else {
        Modo::Controle
    };
    let comando = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(|s| s.as_str())
        .unwrap_or("ajuda");

    match comando {
        "instalar" => instalar(),
        "desinstalar" => desinstalar(),
        "status" => status(),
        "rodar" => rodar(modo),
        _ => {
            ajuda();
            Ok(())
        }
    }
}

fn ajuda() {
    println!(
        "\nfol-discord {}\n\n\
         Uso:\n  \
         fol-discord instalar      liga a correção e faz subir com o Windows\n  \
         fol-discord desinstalar   remove tudo, sem deixar rastro\n  \
         fol-discord status        mostra o estado atual\n  \
         fol-discord rodar         roda em primeiro plano (para depurar)\n\n\
         Opções:\n  \
         --tudo-discord                manda todo domínio do Discord pro exterior\n                                \
         (use só se a correção padrão não bastar)\n",
        env!("CARGO_PKG_VERSION")
    );
}

fn instalar() -> Result<()> {
    let destino = caminho_instalado();
    std::fs::create_dir_all(pasta_dados()).context("criando a pasta de dados")?;

    let atual = std::env::current_exe()?;
    if atual != destino {
        // Se já havia uma cópia rodando, ela precisa sair antes de ser trocada.
        encerrar_outras_instancias();
        std::fs::copy(&atual, &destino).context("copiando o executável")?;
    }

    windows::ativar_autostart(&format!("\"{}\" rodar", destino.display()))
        .context("registrando o autostart")?;
    windows::ativar_pac(&url_pac()).context("ligando o proxy automático")?;

    std::process::Command::new(&destino)
        .arg("rodar")
        .spawn()
        .context("subindo o serviço")?;

    println!("Instalado.\n");
    println!("  executável : {}", destino.display());
    println!("  log        : {}", caminho_log().display());
    println!("  autostart  : sim");
    println!("  PAC        : {}", url_pac());
    println!("\nFeche e abra o Discord uma vez. Depois disso, nunca mais.");
    Ok(())
}

fn desinstalar() -> Result<()> {
    windows::desativar_pac().context("devolvendo o proxy automático")?;
    windows::desativar_autostart().context("removendo o autostart")?;
    encerrar_outras_instancias();
    let _ = std::fs::remove_dir_all(pasta_dados());

    println!("Removido. O proxy automático do Windows voltou ao que era antes.");
    println!("Feche e abra o Discord para ele voltar a sair pelo seu IP normal.");
    Ok(())
}

fn status() -> Result<()> {
    println!("\nfol-discord {}\n", env!("CARGO_PKG_VERSION"));
    println!("  instalado  : {}", sim_nao(caminho_instalado().exists()));
    println!("  autostart  : {}", sim_nao(windows::autostart_ativo()));
    println!("  PAC ligado : {}", sim_nao(windows::pac_ativo(&url_pac())));
    println!("  rodando    : {}", sim_nao(porta_ocupada(PORTA_SOCKS)));
    println!("  log        : {}", caminho_log().display());
    Ok(())
}

/// Encerra cópias antigas do serviço — e só elas. O filtro por PID existe
/// porque o instalador tem o mesmo nome de imagem e mataria a si próprio.
fn encerrar_outras_instancias() {
    let eu = std::process::id();
    let _ = std::process::Command::new("taskkill")
        .args([
            "/F",
            "/IM",
            "fol-discord.exe",
            "/FI",
            &format!("PID ne {eu}"),
        ])
        .output();
    std::thread::sleep(Duration::from_millis(800));
}

fn sim_nao(b: bool) -> &'static str {
    if b {
        "sim"
    } else {
        "não"
    }
}

fn porta_ocupada(porta: u16) -> bool {
    std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{porta}").parse().unwrap(),
        Duration::from_millis(300),
    )
    .is_ok()
}

fn rodar(modo: Modo) -> Result<()> {
    let _ = std::fs::create_dir_all(pasta_dados());
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let piscina = pool::Pool::nova();

        tokio::spawn({
            let p = piscina.clone();
            async move {
                loop {
                    if p.quantidade() < MINIMO_SAUDAVEIS {
                        socks::log::linha("reabastecendo a piscina de proxies...");
                        match p.reabastecer().await {
                            Ok(n) => {
                                socks::log::linha(&format!("{n} proxies estrangeiros validados"));
                                for u in p.listar().iter().take(3) {
                                    socks::log::linha(&format!(
                                        "  {} ({}) {}ms",
                                        u.endereco,
                                        u.regiao,
                                        u.latencia.as_millis()
                                    ));
                                }
                            }
                            Err(e) => socks::log::linha(&format!("falha ao reabastecer: {e}")),
                        }
                    }
                    tokio::time::sleep(INTERVALO_MANUTENCAO).await;
                }
            }
        });

        tokio::spawn(async move {
            if let Err(e) = pac::servir(PORTA_PAC, PORTA_SOCKS).await {
                socks::log::linha(&format!("servidor PAC caiu: {e}"));
            }
        });

        socks::servir(PORTA_SOCKS, piscina, modo).await
    })
}

/// Compilado como aplicativo de janela para não piscar console no autostart.
/// Quando chamado de um terminal, adota o console de quem chamou — e reabre
/// as saídas padrão apontando para ele, senão `println!` escreveria no vazio.
fn anexar_console() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE},
            Storage::FileSystem::{
                CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_SHARE_READ, FILE_SHARE_WRITE,
                OPEN_EXISTING,
            },
            System::Console::{
                AttachConsole, SetStdHandle, ATTACH_PARENT_PROCESS, STD_ERROR_HANDLE,
                STD_OUTPUT_HANDLE,
            },
        };

        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            return; // sem terminal chamador: rodando pelo autostart
        }

        let nome: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
        let saida = CreateFileW(
            nome.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        );
        if saida != INVALID_HANDLE_VALUE {
            SetStdHandle(STD_OUTPUT_HANDLE, saida);
            SetStdHandle(STD_ERROR_HANDLE, saida);
        }
    }
}

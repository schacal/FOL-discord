//! fol-discord — corrige o problema do Discord no Brasil.
//!
//! O Discord decide a região da sua sessão pelo IP que enxerga na abertura.
//! Em vários provedores brasileiros essa decisão sai errada e a transmissão de
//! tela para de funcionar. Este programa faz só o punhado de conexões que
//! determinam essa decisão sair por um IP estrangeiro. A voz, a tela e o resto
//! da internet continuam saindo direto, com o ping de sempre.

#![windows_subsystem = "windows"]

mod discord;
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

    let reiniciar_discord = !args.iter().any(|a| a == "--sem-reiniciar");

    match comando {
        "instalar" => instalar(reiniciar_discord),
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
         fol-discord instalar      liga a correção, reinicia o Discord e sobe com o Windows\n  \
         fol-discord desinstalar   remove tudo, sem deixar rastro\n  \
         fol-discord status        mostra o estado atual\n  \
         fol-discord rodar         roda em primeiro plano (para depurar)\n\n\
         Opções:\n  \
         --sem-reiniciar           não mexe no Discord aberto; a correção vale na\n                            \
         próxima vez que você abrir\n  \
         --tudo-discord            manda todo domínio do Discord pro exterior\n                            \
         (use só se a correção padrão não bastar)\n",
        env!("CARGO_PKG_VERSION")
    );
}

fn instalar(reiniciar_discord: bool) -> Result<()> {
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
    let _ = windows::adicionar_ao_path(&pasta_dados().display().to_string());

    std::process::Command::new(&destino)
        .arg("rodar")
        .spawn()
        .context("subindo o serviço")?;

    // A piscina precisa de alguns segundos para validar os primeiros proxies.
    // Reiniciar o Discord antes disso o faria abrir sem correção.
    print!("Validando proxies");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    for _ in 0..12 {
        std::thread::sleep(Duration::from_secs(5));
        print!(".");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        if porta_ocupada(PORTA_SOCKS) && piscina_pronta() {
            break;
        }
    }
    println!();

    println!("\nInstalado.\n");
    println!("  executável : {}", destino.display());
    println!("  log        : {}", caminho_log().display());
    println!("  autostart  : sim");
    println!("  PAC        : {}", url_pac());

    if reiniciar_discord {
        match discord::reiniciar() {
            Ok(true) => println!("\nDiscord reiniciado. Já está valendo."),
            Ok(false) => println!("\nDiscord não encontrado — a correção vale na próxima vez que você abrir."),
            Err(e) => println!("\nNão consegui reiniciar o Discord ({e}). Feche e abra ele uma vez."),
        }
    } else {
        println!("\nFeche e abra o Discord uma vez.");
    }

    println!("\nEm um terminal novo, o comando `fol-discord` já funciona sozinho.");
    Ok(())
}

/// Lê o log para saber se a primeira validação já terminou. É indireto, mas
/// evita abrir um canal de controle só para isso.
fn piscina_pronta() -> bool {
    std::fs::read_to_string(caminho_log())
        .map(|s| s.contains("proxies estrangeiros validados"))
        .unwrap_or(false)
}

fn desinstalar() -> Result<()> {
    windows::desativar_pac().context("devolvendo o proxy automático")?;
    windows::desativar_autostart().context("removendo o autostart")?;
    let _ = windows::remover_do_path(&pasta_dados().display().to_string());
    encerrar_outras_instancias();

    // Fecha o Discord sem reabrir: reabrir agora, com o proxy já desligado, é
    // exatamente o que o usuário quer — mas deixamos a escolha com ele.
    let estava_aberto = discord::encerrar_se_aberto();
    let _ = std::fs::remove_dir_all(pasta_dados());

    println!("Removido. O proxy automático do Windows voltou ao que era antes.");
    if estava_aberto {
        println!("O Discord foi fechado. Abra de novo e ele já sai pelo seu IP normal.");
    } else {
        println!("Na próxima abertura, o Discord já sai pelo seu IP normal.");
    }
    Ok(())
}

fn status() -> Result<()> {
    println!("\nfol-discord {}\n", env!("CARGO_PKG_VERSION"));
    println!("  instalado  : {}", sim_nao(caminho_instalado().exists()));
    println!("  autostart  : {}", sim_nao(windows::autostart_ativo()));
    println!("  PAC ligado : {}", sim_nao(windows::pac_ativo(&url_pac())));
    println!("  rodando    : {}", sim_nao(porta_ocupada(PORTA_SOCKS)));
    println!(
        "  no PATH    : {}",
        sim_nao(windows::path_ativo(&pasta_dados().display().to_string()))
    );
    println!("  proxies    : {}", sim_nao(piscina_pronta()));
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
                AttachConsole, GetStdHandle, SetStdHandle, ATTACH_PARENT_PROCESS,
                STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
            },
        };

        // Se quem nos chamou já entregou uma saída — um pipe, um arquivo, um
        // `>` —, ela é a saída correta. Sobrescrevê-la pelo console faria
        // `fol-discord status > arquivo` gravar nada.
        let ja_temos = GetStdHandle(STD_OUTPUT_HANDLE);
        if !ja_temos.is_null() && ja_temos != INVALID_HANDLE_VALUE {
            return;
        }

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

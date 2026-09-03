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
mod plataforma;
mod processos;
mod routing;
mod sessao;
mod socks;

use anyhow::{bail, Context, Result};
use routing::Modo;
use std::{ffi::OsStr, path::PathBuf, process::Command, time::Duration};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const PORTA_SOCKS: u16 = 9250;
const PORTA_PAC: u16 = 9251;
const MINIMO_SAUDAVEIS: usize = 3;
const INTERVALO_MANUTENCAO: Duration = Duration::from_secs(300);

/// Passada do vigia da sessão. Curto o bastante para a janela fechar logo
/// depois do silêncio e para o reinício do Discord ser percebido na hora.
const INTERVALO_VIGIA: Duration = Duration::from_secs(1);

#[derive(Debug, PartialEq, Eq)]
struct OpcoesInstalar {
    reiniciar_discord: bool,
    criar_run_legado: bool,
}

fn opcoes_instalar(args: &[String]) -> OpcoesInstalar {
    OpcoesInstalar {
        reiniciar_discord: !args.iter().any(|arg| arg == "--sem-reiniciar"),
        criar_run_legado: !args.iter().any(|arg| arg == "--sem-autostart"),
    }
}

fn manter_arquivos(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--manter-arquivos")
}

fn url_pac() -> String {
    format!("http://127.0.0.1:{PORTA_PAC}/proxy.pac")
}

pub fn pasta_dados() -> PathBuf {
    plataforma::pasta_dados()
}

pub fn caminho_log() -> PathBuf {
    pasta_dados().join("fol.log")
}

/// Marcador escrito pelo serviço quando a piscina tem proxies utilizáveis.
/// Um arquivo, e não uma linha no log: o log sobrevive entre instalações, e
/// procurar texto nele fazia a instalação seguinte se declarar pronta na hora,
/// reiniciando o Discord antes de haver qualquer proxy validado.
pub fn caminho_marcador() -> PathBuf {
    pasta_dados().join("pronto")
}

/// Instante da última passada de manutenção da piscina, em milissegundos de
/// época. É o que a janela mostra em "Última checagem".
///
/// Antes só o botão "Verificar agora" escrevia aqui, então quem nunca clicava
/// via um travessão para sempre — mesmo com o serviço checando a piscina de
/// cinco em cinco minutos desde o boot. O serviço é quem sabe a hora da
/// checagem, então é ele quem carimba.
pub fn caminho_ultima_validacao() -> PathBuf {
    pasta_dados().join("ultima-validacao-ms")
}

fn piscina_pronta() -> bool {
    caminho_marcador().exists()
}

fn milissegundos_agora() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

/// Carimba a passada de manutenção que acabou de terminar. Um disco ocupado
/// não pode derrubar o laço: no pior caso a janela mostra a checagem anterior.
fn registrar_checagem_em(caminho: &std::path::Path, instante: u128) {
    let _ = std::fs::create_dir_all(caminho.parent().unwrap_or(caminho));
    let _ = std::fs::write(caminho, format!("{instante}\n"));
}

fn caminho_instalado() -> PathBuf {
    plataforma::caminho_instalado()
}

/// Todo processo auxiliar nasce sem console. O núcleo é executado tanto pelo
/// PowerShell quanto pela janela; neste último caso, ferramentas como
/// `taskkill` criariam uma caixa preta curta se não receberem esta flag.
fn comando_oculto(programa: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut comando = Command::new(programa);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        comando.creation_flags(CREATE_NO_WINDOW);
    }
    comando
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
        "instalar" => instalar(opcoes_instalar(&args)),
        "desinstalar" => desinstalar(manter_arquivos(&args)),
        "status" => status(),
        "pausar" => pausar(),
        "retomar" => retomar(),
        "abrir-discord" => abrir_discord(&args),
        #[cfg(target_os = "linux")]
        "remover-pacote" => remover_pacote(),
        "reiniciar-discord" => reiniciar_discord(),
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
         fol-discord instalar      liga a correção, reinicia o Discord e sobe com o sistema\n  \
         fol-discord desinstalar   remove tudo, sem deixar rastro\n  \
         fol-discord status        mostra o estado atual\n  \
         fol-discord pausar        pausa a correção sem parar o serviço\n  \
         fol-discord retomar       religa a correção\n  \
         fol-discord abrir-discord abre o Discord pelo launcher gerenciado\n  \
         fol-discord reiniciar-discord fecha e abre só o Discord\n  \
         fol-discord rodar         roda em primeiro plano (para depurar)\n\n\
         Opções:\n  \
         --sem-reiniciar           não mexe no Discord aberto; a correção vale na\n                            \
         próxima vez que você abrir\n  \
         --sem-autostart           não cria a entrada automática (uso do setup)\n  \
         --manter-arquivos         limpa a configuração sem apagar a pasta instalada\n  \
         --tudo-discord            manda todo domínio do Discord pro exterior\n                            \
         (use só se a correção padrão não bastar)\n",
        env!("CARGO_PKG_VERSION")
    );
}

fn instalar(opcoes: OpcoesInstalar) -> Result<()> {
    let destino = caminho_instalado();
    std::fs::create_dir_all(pasta_dados()).context("criando a pasta de dados")?;
    if let Some(pasta) = destino.parent() {
        std::fs::create_dir_all(pasta).context("criando a pasta do executável")?;
    }

    let atual = std::env::current_exe()?;
    if atual != destino {
        // Se já havia uma cópia rodando, ela precisa sair antes de ser trocada.
        encerrar_outras_instancias();
        std::fs::copy(&atual, &destino).context("copiando o executável")?;
        plataforma::preparar_executavel(&destino).context("preparando o executável")?;
    }

    if opcoes.criar_run_legado {
        plataforma::ativar_autostart(&destino)
            .context("registrando o autostart")?;
    }
    plataforma::ativar_pac(&url_pac(), &destino).context("ligando o proxy automático")?;
    let _ = plataforma::registrar_cli(&destino);

    // O marcador é de quem está subindo agora, não da instalação anterior — e
    // a mesma regra vale para a hora da última checagem: mostrar "há 3 d" numa
    // instalação recém-feita seria a janela mentindo sobre si mesma.
    let _ = std::fs::remove_file(caminho_marcador());
    let _ = std::fs::remove_file(caminho_ultima_validacao());

    // Mesmo o processo principal é criado sem console: o instalador pode ter
    // sido chamado pela janela, pelo autostart ou pelo PowerShell.
    comando_oculto(&destino)
        .arg("rodar")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("subindo o serviço")?;

    // A piscina precisa de alguns segundos para validar os primeiros proxies.
    // Reiniciar o Discord antes disso o faria abrir sem correção nenhuma.
    print!("Validando proxies");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let mut pronta = false;
    for _ in 0..15 {
        std::thread::sleep(Duration::from_secs(4));
        print!(".");
        let _ = std::io::Write::flush(&mut std::io::stdout());
        if porta_ocupada(PORTA_SOCKS) && piscina_pronta() {
            pronta = true;
            break;
        }
    }
    println!();
    if !pronta {
        println!("\nNenhum proxy respondeu a tempo. O serviço continua tentando");
        println!("a cada 5 minutos — confira depois com `fol-discord status`.");
    }

    println!("\nInstalado.\n");
    println!("  executável : {}", destino.display());
    println!("  log        : {}", caminho_log().display());
    println!(
        "  autostart  : {}",
        if opcoes.criar_run_legado {
            "sim"
        } else {
            "gerenciado pela interface"
        }
    );
    println!("  PAC        : {}", url_pac());

    if opcoes.reiniciar_discord {
        match discord::reiniciar() {
            Ok(true) => println!("\nDiscord reiniciado. Já está valendo."),
            Ok(false) => println!(
                "\nDiscord não encontrado — a correção vale na próxima vez que você abrir."
            ),
            Err(e) => {
                println!("\nNão consegui reiniciar o Discord ({e}). Feche e abra ele uma vez.")
            }
        }
    } else {
        println!("\nFeche e abra o Discord uma vez.");
    }

    println!("\nEm um terminal novo, o comando `fol-discord` já funciona sozinho.");
    Ok(())
}

fn desinstalar(manter_arquivos: bool) -> Result<()> {
    plataforma::validar_autostart_do_fol(&caminho_instalado())?;
    plataforma::desativar_pac().context("devolvendo o proxy automático")?;
    plataforma::desativar_autostart(&caminho_instalado()).context("removendo o autostart")?;
    let _ = plataforma::remover_cli(&caminho_instalado());
    encerrar_outras_instancias();

    // Fecha o Discord sem reabrir: reabrir agora, com o proxy já desligado, é
    // exatamente o que o usuário quer — mas deixamos a escolha com ele.
    let estava_aberto = discord::encerrar_se_aberto();
    if !manter_arquivos {
        plataforma::remover_arquivos_instalados();
    }

    println!(
        "{} A configuração automática de proxy voltou ao que era antes.",
        if manter_arquivos {
            "Configuração removida; os arquivos foram preservados para o setup."
        } else {
            "Removido."
        }
    );
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
    println!("  autostart  : {}", sim_nao(plataforma::autostart_ativo()));
    println!("  PAC ligado : {}", sim_nao(plataforma::pac_ativo(&url_pac())));
    println!("  rodando    : {}", sim_nao(porta_ocupada(PORTA_SOCKS)));
    println!(
        "  no PATH    : {}",
        sim_nao(plataforma::cli_registrada(&caminho_instalado()))
    );
    println!("  proxies    : {}", sim_nao(piscina_pronta()));
    println!("  log        : {}", caminho_log().display());
    Ok(())
}

/// Reinicia somente o Discord. Não passa pela instalação nem aguarda a
/// validação da piscina: a interface usa este caminho quando o serviço já
/// está em execução.
fn reiniciar_discord() -> Result<()> {
    match discord::reiniciar()? {
        true => println!("Discord reiniciado."),
        false => println!("Discord não encontrado."),
    }
    Ok(())
}

fn pausar() -> Result<()> {
    plataforma::desativar_pac().context("pausando a correção")
}

fn retomar() -> Result<()> {
    if !caminho_instalado().is_file() {
        bail!(
            "o serviço instalado não foi encontrado em {}",
            caminho_instalado().display()
        );
    }
    plataforma::ativar_pac(&url_pac(), &caminho_instalado()).context("retomando a correção")
}

fn abrir_discord(args: &[String]) -> Result<()> {
    if !porta_ocupada(PORTA_SOCKS) {
        let servico = caminho_instalado();
        if !servico.is_file() {
            bail!("o serviço instalado não foi encontrado em {}", servico.display());
        }
        comando_oculto(&servico)
            .arg("rodar")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("subindo o serviço antes do Discord")?;

        for _ in 0..60 {
            if porta_ocupada(PORTA_SOCKS) && piscina_pronta() {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    let extras = args
        .iter()
        .position(|arg| arg == "abrir-discord")
        .map(|indice| &args[indice + 1..])
        .unwrap_or_default();
    if !discord::abrir(extras)? {
        bail!("não encontrei uma instalação nativa ou Flatpak do Discord");
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn remover_pacote() -> Result<()> {
    plataforma::remover_pacote_sistema().context("removendo o pacote do sistema")?;
    desinstalar(false)
}

/// Encerra cópias antigas do serviço — e só elas. O filtro por PID existe
/// porque o instalador tem o mesmo nome de imagem e mataria a si próprio.
fn encerrar_outras_instancias() {
    let eu = std::process::id();
    let antigas: Vec<u32> = processos::pids_por_nome(plataforma::NOME_SERVICO)
        .into_iter()
        .filter(|pid| *pid != eu)
        .collect();
    processos::encerrar_todos(&antigas);
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

/// Vigia a janela de abertura, num fio próprio.
///
/// Fica fora do runtime de propósito: ler a lista de processos do Windows é
/// uma chamada bloqueante, e ela não tem por que disputar uma thread com o
/// tráfego do Discord. Todo o estado da sessão é síncrono, então o fio dá
/// conta sozinho.
fn vigiar_sessao(sessao: std::sync::Arc<sessao::Sessao>, piscina: pool::Pool) {
    std::thread::spawn(move || loop {
        let agora = std::time::Instant::now();

        if sessao.observar_discord(&discord::pids(), agora) {
            socks::log::linha("Discord novo no ar; a correção vale para esta sessão");
        }

        if sessao.avaliar(agora, piscina.quantidade() > 0) {
            // A região já está gravada na sessão. Daqui em diante o Discord
            // fala direto, e quem ficou preso no exterior acabou de cair para
            // reconectar pelo caminho curto.
            socks::log::linha("sessão aberta; o Discord volta a falar direto");
        }

        std::thread::sleep(INTERVALO_VIGIA);
    });
}

fn rodar(modo: Modo) -> Result<()> {
    let _ = std::fs::create_dir_all(pasta_dados());
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async move {
        let piscina = pool::Pool::nova();
        let sessao = std::sync::Arc::new(sessao::Sessao::nova(std::time::Instant::now()));
        vigiar_sessao(sessao.clone(), piscina.clone());

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

                    // O marcador reflete o estado real da piscina: some quando
                    // ela seca, para que `status` não minta.
                    if p.quantidade() > 0 {
                        let _ = std::fs::write(caminho_marcador(), b"");
                    } else {
                        let _ = std::fs::remove_file(caminho_marcador());
                    }

                    // A checagem aconteceu — inclusive quando não achou nada.
                    // Um travessão em "Última checagem" tem que querer dizer
                    // "o serviço não olhou", não "o serviço olhou e falhou".
                    registrar_checagem_em(&caminho_ultima_validacao(), milissegundos_agora());
                    tokio::time::sleep(INTERVALO_MANUTENCAO).await;
                }
            }
        });

        tokio::spawn(async move {
            if let Err(e) = pac::servir(PORTA_PAC, PORTA_SOCKS).await {
                socks::log::linha(&format!("servidor PAC caiu: {e}"));
            }
        });

        socks::servir(PORTA_SOCKS, piscina, modo, sessao).await
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_da_interface_instala_o_servico_sem_run_legado() {
        assert_eq!(
            opcoes_instalar(&[
                "instalar".into(),
                "--sem-reiniciar".into(),
                "--sem-autostart".into(),
            ]),
            OpcoesInstalar {
                reiniciar_discord: false,
                criar_run_legado: false,
            },
        );
    }

    #[test]
    fn cli_sem_novas_opcoes_mantem_o_autostart_legado() {
        assert_eq!(
            opcoes_instalar(&["instalar".into()]),
            OpcoesInstalar {
                reiniciar_discord: true,
                criar_run_legado: true,
            },
        );
    }

    #[test]
    fn desinstalar_com_manter_arquivos_nao_remove_a_pasta() {
        assert!(manter_arquivos(&[
            "desinstalar".into(),
            "--manter-arquivos".into()
        ]));
        assert!(!manter_arquivos(&["desinstalar".into()]));
    }

    #[test]
    fn a_manutencao_carimba_a_hora_que_a_janela_le() {
        let diretorio = tempfile::tempdir().unwrap();
        let caminho = diretorio.path().join("sub").join("ultima-validacao-ms");

        // A pasta ainda não existe: o carimbo tem que criá-la, senão a
        // primeira checagem depois de uma instalação limpa se perde.
        registrar_checagem_em(&caminho, 1_725_000_123_456);

        // O mesmo formato que a ponte da janela sabe ler: milissegundos de
        // época em texto, com a quebra de linha final.
        assert_eq!(
            std::fs::read_to_string(&caminho).unwrap(),
            "1725000123456\n"
        );
        assert_eq!(
            std::fs::read_to_string(&caminho)
                .unwrap()
                .trim()
                .parse::<u64>()
                .unwrap(),
            1_725_000_123_456
        );
    }

    #[test]
    fn a_janela_e_o_servico_apontam_para_o_mesmo_arquivo_de_checagem() {
        assert_eq!(
            caminho_ultima_validacao(),
            pasta_dados().join("ultima-validacao-ms"),
        );
    }
}

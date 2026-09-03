//! Janela de gerenciamento do FOL-discord.
//!
//! A interface é opcional: o serviço sobe com o Windows sozinho e continua
//! corrigindo com esta janela fechada. Aqui só existe a moldura — a bandeja, o
//! esconder ao fechar, e o comando que religa o serviço quando ele caiu.
//!
//! Toda conversa com o serviço acontece do lado do webview, por HTTP em
//! 127.0.0.1:9252. Este processo não fala com a piscina, não lê o `fol.log` e
//! não toca no registro: quem manda nessas coisas continua sendo o serviço.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, WindowEvent, Wry,
};

mod servico;
mod inicializacao;
mod plataforma;
mod processos;

const BANDEJA: &str = "principal";
const JANELA: &str = "principal";

/// Sinal de que fomos abertos pelo serviço no boot: só o ícone, sem janela.
const SEM_JANELA: &str = "--bandeja";

/// Evento que a bandeja manda para o webview, que é quem fala com o serviço.
const EVENTO: &str = "bandeja";

const ICONE_OPERACIONAL: &[u8] = include_bytes!("../icones/bandeja-operacional.png");
const ICONE_PAUSADO: &[u8] = include_bytes!("../icones/bandeja-pausado.png");
const ICONE_SEM_PROXIES: &[u8] = include_bytes!("../icones/bandeja-sem_proxies.png");
const ICONE_PARADO: &[u8] = include_bytes!("../icones/bandeja-parado.png");

/// Guarda o item de menu que troca de nome entre Pausar e Retomar.
struct Bandeja {
    pausar: MenuItem<Wry>,
}

fn icone(estado: &str) -> Image<'static> {
    let bytes = match estado {
        "operacional" => ICONE_OPERACIONAL,
        "pausado" => ICONE_PAUSADO,
        "sem_proxies" | "inicializando" => ICONE_SEM_PROXIES,
        _ => ICONE_PARADO,
    };
    Image::from_bytes(bytes).expect("ícone da bandeja embutido é inválido")
}

/// O que aparece ao passar o mouse na bandeja — a resposta sem abrir nada.
fn dica(estado: &str) -> &'static str {
    match estado {
        "operacional" => "FOL-discord — funcionando",
        "pausado" => "FOL-discord — pausado",
        "sem_proxies" => "FOL-discord — procurando saída",
        "inicializando" => "FOL-discord — preparando",
        _ => "FOL-discord — parado",
    }
}

/// Traz a janela para a frente — pela bandeja ou por uma segunda abertura — e
/// aproveita para perguntar se saiu versão nova, porque é agora que a pessoa
/// está olhando. A folga entre consultas fica com o serviço.
fn mostrar(app: &AppHandle) {
    if let Some(janela) = app.get_webview_window(JANELA) {
        let _ = janela.show();
        let _ = janela.unminimize();
        let _ = janela.set_focus();
    }
    servico::verificar_atualizacao_ao_mostrar();
}

/// Sobe (ou reinstala) a cópia estável do serviço a partir do arquivo que o
/// instalador deixou ao lado desta janela. A janela não fala com nenhuma API
/// que o serviço estável não expõe.
#[tauri::command]
fn religar_servico() -> Result<(), String> {
    servico::garantir_servico(false, false)
}

#[tauri::command]
fn status_servico() -> servico::Status {
    servico::status()
}

#[tauri::command]
fn conexoes_servico() -> Vec<servico::Conexao> {
    servico::conexoes()
}

#[tauri::command]
fn pausar_servico() -> Result<(), String> {
    servico::pausar()
}

#[tauri::command]
fn retomar_servico() -> Result<(), String> {
    servico::retomar()
}

#[tauri::command]
fn verificar_servico() -> Result<servico::Verificacao, String> {
    servico::verificar()
}

#[tauri::command]
fn definir_autostart(ligado: bool) -> Result<(), String> {
    servico::definir_autostart(ligado)
}

#[tauri::command]
fn reiniciar_discord() -> Result<bool, String> {
    servico::reiniciar_discord()
}

#[tauri::command]
fn atualizar_servico() -> Result<String, String> {
    servico::url_da_atualizacao()
}

#[tauri::command]
fn iniciar_desinstalacao(app: AppHandle) -> Result<(), String> {
    let mut comando = servico::comando_desinstalador()?;
    comando
        .spawn()
        .map_err(|erro| format!("não consegui abrir o desinstalador: {erro}"))?;
    app.exit(0);
    Ok(())
}

/// O webview avisa o estado; a bandeja muda de cor. É a resposta para quem não
/// quer abrir a janela para saber se está funcionando.
#[tauri::command]
fn definir_estado_bandeja(app: AppHandle, estado: String, pausado: bool) -> Result<(), String> {
    if let Some(bandeja) = app.tray_by_id(BANDEJA) {
        bandeja
            .set_icon(Some(icone(&estado)))
            .map_err(|e| e.to_string())?;
        let _ = bandeja.set_tooltip(Some(dica(&estado)));
    }

    // `state` puro entra em pânico quando o estado ainda não foi registrado, e
    // com `panic = "abort"` isso derruba o programa inteiro por causa do texto
    // de um item de menu. O webview pode invocar antes de `setup` terminar.
    match app.try_state::<Bandeja>() {
        Some(bandeja) => bandeja
            .pausar
            .set_text(if pausado { "Retomar" } else { "Pausar" })
            .map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

fn main() {
    let boot = std::env::args().any(|arg| arg == SEM_JANELA);
    tauri::Builder::default()
        // Duas janelas conversando com o mesmo serviço só confundem. A segunda
        // abertura traz a primeira para a frente e morre.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            mostrar(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            religar_servico,
            status_servico,
            conexoes_servico,
            pausar_servico,
            retomar_servico,
            verificar_servico,
            definir_autostart,
            reiniciar_discord,
            atualizar_servico,
            iniciar_desinstalacao,
            definir_estado_bandeja
        ])
        .setup(move |app| {
            servico::iniciar_verificacao_atualizacao();
            let abrir = MenuItem::with_id(app, "abrir", "Abrir", true, None::<&str>)?;
            let pausar = MenuItem::with_id(app, "pausar", "Pausar", true, None::<&str>)?;
            let separador = PredefinedMenuItem::separator(app)?;
            // O rótulo diz o que acontece: sair fecha a janela, não a correção.
            let sair = MenuItem::with_id(
                app,
                "sair",
                "Sair (o serviço continua)",
                true,
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&abrir, &pausar, &separador, &sair])?;

            // A bandeja nasce só aqui. `app.trayIcon` no `tauri.conf.json`
            // criaria uma segunda antes deste `setup`, com o mesmo id e sem
            // menu nenhum — duas no relógio, e `tray_by_id` devolvendo a
            // errada, que é a que ficaria vermelha para sempre.
            TrayIconBuilder::with_id(BANDEJA)
                .icon(icone("inicializando"))
                .tooltip(dica("inicializando"))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, evento| match evento.id().as_ref() {
                    "abrir" => mostrar(app),
                    // Quem fala com o serviço é o webview, sempre.
                    "pausar" => {
                        let _ = app.emit(EVENTO, "alternar-pausa");
                    }
                    "sair" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|bandeja, evento| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = evento
                    {
                        mostrar(bandeja.app_handle());
                    }
                })
                .build(app)?;

            app.manage(Bandeja { pausar });

            // A bandeja já está visível quando o serviço começa a trabalhar.
            // No boot ele nunca reinicia Discord e nem recria a tarefa; na
            // primeira abertura instalada ele preserva o comportamento atual
            // de reiniciar uma vez depois de iniciar a correção.
            std::thread::spawn(move || {
                if let Err(erro) = servico::garantir_servico(!boot, false) {
                    servico::registrar_erro_inicializacao(erro);
                    return;
                }

                let Ok(interface) = std::env::current_exe() else {
                    return;
                };
                if boot {
                    let _ = inicializacao::tarefa_ativa(&interface);
                } else if inicializacao::interface_instalada(&interface) {
                    if let Err(erro) = inicializacao::ativar_tarefa(
                        &interface,
                        &servico::executavel_instalado(),
                    ) {
                        servico::registrar_erro_inicializacao(erro);
                    }
                }
            });

            if let Some(janela) = app.get_webview_window(JANELA) {
                // Fechar esconde para a bandeja. Sair de verdade só pelo menu.
                let escondivel = janela.clone();
                janela.on_window_event(move |evento| {
                    if let WindowEvent::CloseRequested { api, .. } = evento {
                        api.prevent_close();
                        let _ = escondivel.hide();
                    }
                });

                // No boot o serviço nos chama com --bandeja: nada de janela
                // piscando na cara de quem acabou de ligar o computador.
                if !boot {
                    let _ = janela.show();
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("a janela do FOL-discord não subiu");
}

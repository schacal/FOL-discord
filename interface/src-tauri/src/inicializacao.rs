//! Contratos puros para a inicialização da interface instalada.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use winreg::{enums::*, RegKey};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub const TAREFA_BANDEJA: &str = "FolDiscord.Bandeja";

const CHAVE_RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Onde o setup NSIS registra o desinstalador, em ordem de tentativa.
///
/// O template do Tauri usa `productName` na chave (`UNINSTKEY`), não o
/// `identifier`. Procurar só pelo identificador nunca achava nada, e o botão
/// Desinstalar da janela instalada morria em "não encontrei o desinstalador"
/// — que é exatamente o defeito que este vetor conserta. O identificador fica
/// como segunda tentativa para instalações antigas.
const CHAVES_DESINSTALAR: &[&str] = &[
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\FOL-discord",
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\br.com.foldiscord.janela",
];

/// O NSIS sempre grava o desinstalador com este nome, ao lado do executável.
const NOME_DESINSTALADOR: &str = "uninstall.exe";
const MARCADOR_INSTALACAO: &str = ".fol-discord-instalada";
const NOME_RUN: &str = "FolDiscord";

#[derive(Debug, PartialEq, Eq)]
struct TarefaXml {
    xml: String,
}

fn xml_escapado(valor: &str) -> String {
    valor
        .chars()
        .flat_map(|caractere| match caractere {
            '&' => "&amp;".chars().collect::<Vec<_>>(),
            '<' => "&lt;".chars().collect(),
            '>' => "&gt;".chars().collect(),
            '\"' => "&quot;".chars().collect(),
            '\'' => "&apos;".chars().collect(),
            _ => vec![caractere],
        })
        .collect()
}

fn xml_tarefa(path_ui: &Path, usuario: &str) -> TarefaXml {
    let usuario = xml_escapado(usuario);
    let caminho = xml_escapado(&path_ui.display().to_string());
    TarefaXml {
        xml: format!(
            "<?xml version=\"1.0\" encoding=\"UTF-16\"?><Task version=\"1.4\" xmlns=\"http://schemas.microsoft.com/windows/2004/02/mit/task\"><Triggers><LogonTrigger><Enabled>true</Enabled><UserId>{usuario}</UserId></LogonTrigger></Triggers><Principals><Principal id=\"Author\"><UserId>{usuario}</UserId><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals><Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><AllowStartOnDemand>true</AllowStartOnDemand><Enabled>true</Enabled><Hidden>false</Hidden></Settings><Actions Context=\"Author\"><Exec><Command>{caminho}</Command><Arguments>--bandeja</Arguments></Exec></Actions></Task>"
        ),
    }
}

fn entrada_run_e_do_fol(valor: &str, servico: &Path) -> bool {
    let esperado = servico.to_string_lossy();
    let valor = valor.trim();

    let restante = valor
        .strip_prefix('"')
        .and_then(|sem_abertura| sem_abertura.strip_prefix(esperado.as_ref()));

    matches!(restante, Some(restante) if restante.starts_with('"'))
}

pub fn interface_instalada(atual: &Path) -> bool {
    atual
        .parent()
        .is_some_and(|pasta| pasta.join(MARCADOR_INSTALACAO).is_file())
}

fn caminho_igual(esquerda: &Path, direita: &Path) -> bool {
    esquerda
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .eq_ignore_ascii_case(direita.to_string_lossy().trim_end_matches(['\\', '/']))
}

fn uninstaller_pertence_a_instalacao(interface: &Path, uninstaller: &Path) -> bool {
    interface
        .parent()
        .zip(uninstaller.parent())
        .is_some_and(|(pasta_interface, pasta_uninstaller)| caminho_igual(pasta_interface, pasta_uninstaller))
}

fn separar_linha_de_comando(linha: &str) -> Result<(PathBuf, Vec<String>), String> {
    let linha = linha.trim();
    if linha.is_empty() {
        return Err("o desinstalador registrado está vazio".into());
    }

    let (executavel, restante) = if let Some(sem_abertura) = linha.strip_prefix('"') {
        let Some(fim) = sem_abertura.find('"') else {
            return Err("o caminho do desinstalador tem aspas sem fechamento".into());
        };
        (&sem_abertura[..fim], &sem_abertura[fim + 1..])
    } else {
        let fim = linha.find(char::is_whitespace).unwrap_or(linha.len());
        (&linha[..fim], &linha[fim..])
    };

    if executavel.is_empty() {
        return Err("o caminho do desinstalador está vazio".into());
    }

    let mut argumentos = Vec::new();
    let mut restante = restante.trim_start();
    while !restante.is_empty() {
        let (argumento, depois) = if let Some(sem_abertura) = restante.strip_prefix('"') {
            let Some(fim) = sem_abertura.find('"') else {
                return Err("um argumento do desinstalador tem aspas sem fechamento".into());
            };
            (&sem_abertura[..fim], &sem_abertura[fim + 1..])
        } else {
            let fim = restante.find(char::is_whitespace).unwrap_or(restante.len());
            (&restante[..fim], &restante[fim..])
        };
        argumentos.push(argumento.to_string());
        restante = depois.trim_start();
    }

    Ok((PathBuf::from(executavel), argumentos))
}

fn xml_da_tarefa_corresponde(xml: &str, interface: &Path) -> bool {
    let comando = xml_escapado(&interface.display().to_string());
    xml.contains(&format!("<Command>{comando}</Command>"))
        && xml.contains("<Arguments>--bandeja</Arguments>")
}

fn comando_oculto(programa: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut comando = Command::new(programa);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        comando.creation_flags(CREATE_NO_WINDOW);
    }
    comando
}

fn texto_da_saida(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.iter().step_by(2).any(|byte| *byte == 0) {
        let palavras: Vec<u16> = bytes
            .chunks_exact(2)
            .skip(usize::from(bytes.starts_with(&[0xff, 0xfe])))
            .map(|par| u16::from_le_bytes([par[0], par[1]]))
            .collect();
        String::from_utf16_lossy(&palavras)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn tarefa_existe() -> bool {
    comando_oculto("schtasks")
        .args(["/query", "/tn", TAREFA_BANDEJA])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub fn tarefa_ativa(interface: &Path) -> bool {
    let Ok(saida) = comando_oculto("schtasks")
        .args(["/query", "/tn", TAREFA_BANDEJA, "/xml"])
        .output()
    else {
        return false;
    };

    saida.status.success() && xml_da_tarefa_corresponde(&texto_da_saida(&saida.stdout), interface)
}

fn hkcu() -> RegKey {
    RegKey::predef(HKEY_CURRENT_USER)
}

fn valor_run_legado() -> Result<Option<String>, String> {
    let chave = match hkcu().open_subkey(CHAVE_RUN) {
        Ok(chave) => chave,
        Err(erro) if erro.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(erro) => return Err(format!("não consegui ler a inicialização do Windows: {erro}")),
    };
    match chave.get_value::<String, _>(NOME_RUN) {
        Ok(valor) => Ok(Some(valor)),
        Err(erro) if erro.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(erro) => Err(format!("não consegui ler a inicialização do Windows: {erro}")),
    }
}

fn validar_run_legado(servico: &Path) -> Result<(), String> {
    match valor_run_legado()? {
        Some(valor) if !entrada_run_e_do_fol(&valor, servico) => Err(
            "A entrada de inicialização FolDiscord pertence a outro programa e não foi alterada."
                .into(),
        ),
        _ => Ok(()),
    }
}

fn remover_run_legado_do_fol(servico: &Path) -> Result<(), String> {
    let Some(valor) = valor_run_legado()? else {
        return Ok(());
    };
    if !entrada_run_e_do_fol(&valor, servico) {
        return Err(
            "A entrada de inicialização FolDiscord pertence a outro programa e não foi alterada."
                .into(),
        );
    }

    hkcu()
        .open_subkey_with_flags(CHAVE_RUN, KEY_WRITE)
        .map_err(|erro| format!("não consegui abrir a inicialização do Windows: {erro}"))?
        .delete_value(NOME_RUN)
        .map_err(|erro| format!("não consegui remover o autostart legado: {erro}"))
}

fn caminho_xml_temporario() -> PathBuf {
    let instante = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!(
        "fol-discord-bandeja-{}-{instante}.xml",
        std::process::id()
    ))
}

pub fn ativar_tarefa(interface: &Path, servico: &Path) -> Result<(), String> {
    if !interface.is_absolute() || !interface_instalada(interface) {
        return Err("a inicialização automática só pode usar a interface instalada pelo setup".into());
    }
    validar_run_legado(servico)?;

    let dominio = env::var("USERDOMAIN")
        .map_err(|_| "não encontrei o domínio do usuário atual".to_string())?;
    let usuario = env::var("USERNAME")
        .map_err(|_| "não encontrei o nome do usuário atual".to_string())?;
    let xml = xml_tarefa(interface, &format!(r"{dominio}\{usuario}"));
    let caminho_xml = caminho_xml_temporario();
    fs::write(&caminho_xml, xml.xml)
        .map_err(|erro| format!("não consegui preparar a tarefa de inicialização: {erro}"))?;

    let resultado = comando_oculto("schtasks")
        .args([
            "/create",
            "/tn",
            TAREFA_BANDEJA,
            "/xml",
            &caminho_xml.to_string_lossy(),
            "/f",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output();
    let _ = fs::remove_file(&caminho_xml);

    let resultado = resultado.map_err(|erro| format!("não consegui registrar a tarefa: {erro}"))?;
    if !resultado.status.success() {
        return Err(format!(
            "não consegui registrar a tarefa de inicialização: {}",
            texto_da_saida(&resultado.stderr).trim()
        ));
    }
    remover_run_legado_do_fol(servico)
}

pub fn desativar_tarefa() -> Result<(), String> {
    if !tarefa_existe() {
        return Ok(());
    }

    let resultado = comando_oculto("schtasks")
        .args(["/delete", "/tn", TAREFA_BANDEJA, "/f"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|erro| format!("não consegui remover a tarefa de inicialização: {erro}"))?;
    if !resultado.status.success() {
        return Err(format!(
            "não consegui remover a tarefa de inicialização: {}",
            texto_da_saida(&resultado.stderr).trim()
        ));
    }
    if tarefa_existe() {
        return Err("a tarefa de inicialização continuou registrada".into());
    }
    Ok(())
}

fn linha_de_comando_registrada() -> Option<String> {
    CHAVES_DESINSTALAR.iter().find_map(|chave| {
        let linha: String = hkcu().open_subkey(chave).ok()?.get_value("UninstallString").ok()?;
        (!linha.trim().is_empty()).then_some(linha)
    })
}

/// O desinstalador que o setup gravou ao lado desta interface.
///
/// A chave do registro é a fonte preferida — ela carrega os argumentos que o
/// setup escolheu. Mas a janela instalada não pode ficar sem botão só porque
/// alguém limpou o "Adicionar ou remover programas": o `uninstall.exe` ao lado
/// do executável é o mesmo arquivo, e passa pela mesma conferência de pasta
/// que já protegia o caminho do registro.
fn desinstalador_e_argumentos(
    interface: &Path,
    registrado: Option<String>,
) -> Result<(PathBuf, Vec<String>), String> {
    match registrado {
        Some(linha) => separar_linha_de_comando(&linha),
        None => {
            let vizinho = interface
                .parent()
                .ok_or_else(|| "não encontrei a pasta da interface".to_string())?
                .join(NOME_DESINSTALADOR);
            if !vizinho.is_file() {
                return Err("não encontrei o desinstalador registrado pelo setup".into());
            }
            Ok((vizinho, Vec::new()))
        }
    }
}

pub fn comando_desinstalador() -> Result<Command, String> {
    let interface = env::current_exe().map_err(|erro| format!("não encontrei a interface atual: {erro}"))?;
    let (uninstaller, argumentos) =
        desinstalador_e_argumentos(&interface, linha_de_comando_registrada())?;
    if !uninstaller.is_absolute() {
        return Err("o desinstalador registrado não tem caminho absoluto".into());
    }

    let interface = fs::canonicalize(&interface)
        .map_err(|erro| format!("não consegui validar a pasta da interface: {erro}"))?;
    let uninstaller = fs::canonicalize(&uninstaller)
        .map_err(|erro| format!("não consegui validar o desinstalador registrado: {erro}"))?;
    if !uninstaller_pertence_a_instalacao(&interface, &uninstaller) {
        return Err("o desinstalador registrado não pertence à instalação atual do FOL-discord".into());
    }

    let mut comando = comando_oculto(uninstaller);
    comando.args(argumentos);
    Ok(comando)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn tarefa_de_logon_abre_a_interface_instalada_na_bandeja() {
        let tarefa = xml_tarefa(
            Path::new(r"C:\Users\Ana\AppData\Local\FOL-discord\fol-discord-janela.exe"),
            r"PC-ANA\Ana",
        );
        assert!(tarefa.xml.contains("<LogonTrigger>"));
        assert!(tarefa.xml.contains("<UserId>PC-ANA\\Ana</UserId>"));
        assert!(tarefa.xml.contains("<LogonType>InteractiveToken</LogonType>"));
        assert!(tarefa.xml.contains("<RunLevel>LeastPrivilege</RunLevel>"));
        assert!(tarefa.xml.contains("<AllowStartOnDemand>true</AllowStartOnDemand>"));
        assert!(tarefa
            .xml
            .contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
        assert!(tarefa.xml.contains(
            "<Command>C:\\Users\\Ana\\AppData\\Local\\FOL-discord\\fol-discord-janela.exe</Command>"
        ));
        assert!(tarefa.xml.contains("<Arguments>--bandeja</Arguments>"));
    }

    #[test]
    fn nao_confunde_uma_entrada_run_de_terceiro_com_a_do_fol() {
        let servico = Path::new(r"C:\Users\Ana\AppData\Local\FolDiscord\fol-discord.exe");
        assert!(entrada_run_e_do_fol(
            r#""C:\Users\Ana\AppData\Local\FolDiscord\fol-discord.exe" rodar"#,
            servico,
        ));
        assert!(!entrada_run_e_do_fol(r"C:\Outro\fol-discord.exe rodar", servico));
    }

    #[test]
    fn somente_o_setup_marcado_pode_registrar_o_boot() {
        let root = tempfile::tempdir().unwrap();
        let exe = root.path().join("fol-discord-janela.exe");
        std::fs::write(&exe, b"").unwrap();
        assert!(!interface_instalada(&exe));
        std::fs::write(root.path().join(".fol-discord-instalada"), b"nsis\n").unwrap();
        assert!(interface_instalada(&exe));
    }

    #[test]
    fn desinstalador_entre_aspas_preserva_argumentos() {
        let (exe, args) = separar_linha_de_comando(
            r#""C:\Users\Ana\AppData\Local\FOL-discord\uninstall.exe" /S"#,
        )
        .unwrap();
        assert_eq!(
            exe,
            PathBuf::from(r"C:\Users\Ana\AppData\Local\FOL-discord\uninstall.exe")
        );
        assert_eq!(args, vec!["/S"]);
    }

    #[test]
    fn xml_escapa_um_caminho_de_usuario_valido() {
        let tarefa = xml_tarefa(
            Path::new(r"C:\Users\A&B\fol-discord-janela.exe"),
            r"PC\A&B",
        );
        assert!(tarefa.xml.contains("A&amp;B"));
        assert!(!tarefa.xml.contains("A&B</"));
    }

    #[test]
    fn procura_o_desinstalador_pela_chave_que_o_setup_realmente_grava() {
        // O template NSIS do Tauri monta `UNINSTKEY` com o `productName`. Foi
        // por procurar só pelo `identifier` que o botão Desinstalar parou de
        // funcionar depois de instalar pelo setup.
        assert_eq!(
            CHAVES_DESINSTALAR[0],
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall\FOL-discord"
        );
        assert!(CHAVES_DESINSTALAR
            .contains(&r"Software\Microsoft\Windows\CurrentVersion\Uninstall\br.com.foldiscord.janela"));
    }

    #[test]
    fn sem_registro_usa_o_desinstalador_ao_lado_da_interface() {
        let root = tempfile::tempdir().unwrap();
        let interface = root.path().join("fol-discord-janela.exe");
        std::fs::write(&interface, b"").unwrap();

        assert!(
            desinstalador_e_argumentos(&interface, None).is_err(),
            "sem uninstall.exe ao lado não há desinstalador nenhum para abrir"
        );

        let vizinho = root.path().join("uninstall.exe");
        std::fs::write(&vizinho, b"").unwrap();
        let (encontrado, argumentos) = desinstalador_e_argumentos(&interface, None).unwrap();
        assert_eq!(encontrado, vizinho);
        assert!(argumentos.is_empty());
    }

    #[test]
    fn o_registro_tem_precedencia_sobre_o_vizinho() {
        let root = tempfile::tempdir().unwrap();
        let interface = root.path().join("fol-discord-janela.exe");
        std::fs::write(&interface, b"").unwrap();
        std::fs::write(root.path().join("uninstall.exe"), b"").unwrap();

        let (encontrado, argumentos) = desinstalador_e_argumentos(
            &interface,
            Some(r#""C:\Instalado\uninstall.exe" /P"#.to_string()),
        )
        .unwrap();

        assert_eq!(encontrado, PathBuf::from(r"C:\Instalado\uninstall.exe"));
        assert_eq!(argumentos, vec!["/P"]);
    }

    #[test]
    fn recusa_um_desinstalador_fora_da_pasta_da_interface() {
        let interface = Path::new(r"C:\Users\Ana\AppData\Local\FOL-discord\fol-discord-janela.exe");
        let dentro = Path::new(r"C:\Users\Ana\AppData\Local\FOL-discord\uninstall.exe");
        let fora = Path::new(r"C:\Users\Ana\Downloads\uninstall.exe");

        assert!(uninstaller_pertence_a_instalacao(interface, dentro));
        assert!(!uninstaller_pertence_a_instalacao(interface, fora));
    }

    #[test]
    fn tarefa_com_mesmo_nome_e_outra_acao_nao_e_ativa() {
        let interface = Path::new(r"C:\Users\Ana\AppData\Local\FOL-discord\fol-discord-janela.exe");
        let tarefa = xml_tarefa(interface, r"PC-ANA\Ana");
        let outra_acao = tarefa.xml.replace(
            "fol-discord-janela.exe",
            "programa-de-terceiro.exe",
        );

        assert!(xml_da_tarefa_corresponde(&tarefa.xml, interface));
        assert!(!xml_da_tarefa_corresponde(&outra_acao, interface));
    }
}

//! Integração Linux sem depender de um ambiente gráfico específico.
//!
//! O núcleo vive no perfil do usuário. O launcher do Discord recebe o PAC por
//! argumento do Chromium, e o autostart segue a especificação XDG.

use anyhow::{bail, Context, Result};
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
};

pub const NOME_SERVICO: &str = "fol-discord";
const ASSINATURA: &str = "X-FOL-Discord-Managed=true";

fn variavel_caminho(nome: &str) -> Option<PathBuf> {
    std::env::var_os(nome)
        .filter(|valor| !valor.is_empty())
        .map(PathBuf::from)
}

fn home() -> PathBuf {
    variavel_caminho("HOME").unwrap_or_else(|| {
        std::env::temp_dir().join(format!("fol-discord-{}", unsafe { libc::geteuid() }))
    })
}

fn xdg_estado() -> PathBuf {
    variavel_caminho("XDG_STATE_HOME").unwrap_or_else(|| home().join(".local/state"))
}

fn xdg_dados() -> PathBuf {
    variavel_caminho("XDG_DATA_HOME").unwrap_or_else(|| home().join(".local/share"))
}

fn xdg_configuracao() -> PathBuf {
    variavel_caminho("XDG_CONFIG_HOME").unwrap_or_else(|| home().join(".config"))
}

pub fn pasta_dados() -> PathBuf {
    xdg_estado().join("fol-discord")
}

fn pasta_instalacao() -> PathBuf {
    xdg_dados().join("fol-discord")
}

pub fn caminho_instalado() -> PathBuf {
    pasta_instalacao().join(NOME_SERVICO)
}

fn pasta_configuracao() -> PathBuf {
    xdg_configuracao().join("fol-discord")
}

fn marcador_pac() -> PathBuf {
    pasta_configuracao().join("pac-url")
}

fn launcher_discord() -> PathBuf {
    xdg_dados()
        .join("applications")
        .join("fol-discord-discord.desktop")
}

fn autostart() -> PathBuf {
    xdg_configuracao()
        .join("autostart")
        .join("fol-discord-core.desktop")
}

fn atalho_cli() -> PathBuf {
    home().join(".local/bin").join(NOME_SERVICO)
}

fn escapar_exec(valor: &str) -> String {
    let mut saida = String::with_capacity(valor.len() + 2);
    saida.push('"');
    for caractere in valor.chars() {
        match caractere {
            '\\' | '"' | '`' | '$' => {
                saida.push('\\');
                saida.push(caractere);
            }
            '%' => saida.push_str("%%"),
            _ => saida.push(caractere),
        }
    }
    saida.push('"');
    saida
}

fn desktop_discord(servico: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Discord (FOL-discord)\nComment=Abre o Discord com a correção regional\nExec={} abrir-discord %U\nIcon=discord\nTerminal=false\nCategories=Network;InstantMessaging;\nMimeType=x-scheme-handler/discord;\nStartupWMClass=discord\n{}\n",
        escapar_exec(&servico.display().to_string()),
        ASSINATURA
    )
}

fn desktop_autostart(servico: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=FOL-discord\nComment=Inicia a correção regional do Discord\nExec={} rodar\nIcon=fol-discord\nTerminal=false\nX-GNOME-Autostart-enabled=true\n{}\n",
        escapar_exec(&servico.display().to_string()),
        ASSINATURA
    )
}

fn e_gerenciado(caminho: &Path) -> bool {
    fs::read_to_string(caminho)
        .map(|texto| texto.lines().any(|linha| linha == ASSINATURA))
        .unwrap_or(false)
}

fn gravar_gerenciado(caminho: &Path, conteudo: &str) -> Result<()> {
    let pasta = caminho.parent().context("caminho gerenciado sem pasta")?;
    fs::create_dir_all(pasta)?;
    if caminho.exists() && !e_gerenciado(caminho) {
        bail!(
            "recusei sobrescrever o arquivo não gerenciado {}",
            caminho.display()
        );
    }
    fs::write(caminho, conteudo)?;
    Ok(())
}

pub fn preparar_executavel(caminho: &Path) -> Result<()> {
    let mut permissoes = fs::metadata(caminho)?.permissions();
    permissoes.set_mode(permissoes.mode() | 0o700);
    fs::set_permissions(caminho, permissoes)?;
    Ok(())
}

pub fn remover_arquivos_instalados() {
    let _ = fs::remove_dir_all(pasta_dados());
    let executavel = caminho_instalado();
    let _ = fs::remove_file(&executavel);
    if let Some(pasta) = executavel.parent() {
        let _ = fs::remove_dir(pasta);
    }
}

/// Ativa a correção sem alterar o proxy global do desktop. O atalho gerenciado
/// chama o próprio núcleo, que abre o Discord com `--proxy-pac-url`.
pub fn ativar_pac(url: &str, servico: &Path) -> Result<()> {
    gravar_gerenciado(&launcher_discord(), &desktop_discord(servico))
        .context("criando o launcher gerenciado do Discord")?;
    fs::create_dir_all(pasta_configuracao())?;
    fs::write(marcador_pac(), format!("{url}\n"))?;
    Ok(())
}

pub fn desativar_pac() -> Result<()> {
    let launcher = launcher_discord();
    if launcher.exists() {
        if !e_gerenciado(&launcher) {
            bail!(
                "o launcher {} não pertence ao FOL-discord",
                launcher.display()
            );
        }
        fs::remove_file(launcher)?;
    }
    match fs::remove_file(marcador_pac()) {
        Ok(()) => Ok(()),
        Err(erro) if erro.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(erro) => Err(erro.into()),
    }
}

pub fn pac_ativo(url: &str) -> bool {
    e_gerenciado(&launcher_discord())
        && fs::read_to_string(marcador_pac())
            .map(|valor| valor.trim() == url)
            .unwrap_or(false)
}

pub fn ativar_autostart(servico: &Path) -> Result<()> {
    gravar_gerenciado(&autostart(), &desktop_autostart(servico))
        .context("criando a entrada XDG de autostart")
}

pub fn validar_autostart_do_fol(_servico: &Path) -> Result<()> {
    let caminho = autostart();
    if caminho.exists() && !e_gerenciado(&caminho) {
        bail!(
            "a entrada de autostart {} não pertence ao FOL-discord",
            caminho.display()
        );
    }
    Ok(())
}

pub fn desativar_autostart(servico: &Path) -> Result<()> {
    validar_autostart_do_fol(servico)?;
    match fs::remove_file(autostart()) {
        Ok(()) => Ok(()),
        Err(erro) if erro.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(erro) => Err(erro.into()),
    }
}

pub fn autostart_ativo() -> bool {
    e_gerenciado(&autostart())
}

pub fn registrar_cli(servico: &Path) -> Result<()> {
    let atalho = atalho_cli();
    fs::create_dir_all(atalho.parent().context("atalho sem pasta")?)?;
    if let Ok(destino) = fs::read_link(&atalho) {
        if destino == servico {
            return Ok(());
        }
        bail!(
            "o atalho {} já aponta para outro programa",
            atalho.display()
        );
    }
    if atalho.exists() {
        bail!("recusei sobrescrever {}", atalho.display());
    }
    symlink(servico, atalho)?;
    Ok(())
}

pub fn remover_cli(servico: &Path) -> Result<()> {
    let atalho = atalho_cli();
    match fs::read_link(&atalho) {
        Ok(destino) if destino == servico => fs::remove_file(atalho).map_err(Into::into),
        Ok(_) => bail!("o atalho {} não pertence ao FOL-discord", atalho.display()),
        Err(erro) if erro.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(erro) => Err(erro.into()),
    }
}

pub fn cli_registrada(servico: &Path) -> bool {
    fs::read_link(atalho_cli())
        .map(|destino| destino == servico)
        .unwrap_or(false)
}

fn programa_sistema(nome: &str) -> Option<PathBuf> {
    ["/usr/bin", "/bin", "/usr/sbin", "/sbin"]
        .into_iter()
        .map(|pasta| Path::new(pasta).join(nome))
        .find(|candidato| candidato.is_file())
}

fn familia_em(texto: &str) -> String {
    texto
        .lines()
        .filter_map(|linha| {
            let (nome, valor) = linha.split_once('=')?;
            matches!(nome, "ID" | "ID_LIKE").then(|| valor.trim_matches('"').to_ascii_lowercase())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn familia_distribuicao() -> String {
    familia_em(&fs::read_to_string("/etc/os-release").unwrap_or_default())
}

fn candidatos_da_familia(familia: &str) -> &'static [(&'static str, &'static [&'static str])] {
    const APT: &[&str] = &["remove", "--yes", "fol-discord"];
    const DNF: &[&str] = &["remove", "--assumeyes", "fol-discord"];
    const PACMAN: &[&str] = &["--remove", "--noconfirm", "fol-discord"];
    const ZYPPER: &[&str] = &["--non-interactive", "remove", "fol-discord"];

    if familia.contains("arch") {
        &[("pacman", PACMAN)]
    } else if familia.contains("suse") {
        &[("zypper", ZYPPER)]
    } else if familia.contains("fedora") || familia.contains("rhel") {
        &[("dnf", DNF)]
    } else if familia.contains("debian") || familia.contains("ubuntu") {
        &[("apt-get", APT)]
    } else {
        &[
            ("apt-get", APT),
            ("dnf", DNF),
            ("pacman", PACMAN),
            ("zypper", ZYPPER),
        ]
    }
}

fn comando_do_gerenciador() -> Option<(PathBuf, &'static [&'static str])> {
    candidatos_da_familia(&familia_distribuicao())
        .iter()
        .find_map(|(programa, argumentos)| programa_sistema(programa).map(|p| (p, *argumentos)))
}

/// Remove primeiro o pacote que contém a interface. O processo atual é a
/// cópia por usuário do núcleo, portanto continua vivo até poder limpar XDG e
/// o launcher depois que o gerenciador termina.
pub fn remover_pacote_sistema() -> Result<()> {
    if let Some(appimage) = variavel_caminho("APPIMAGE") {
        let e_appimage = appimage.is_absolute()
            && appimage.is_file()
            && appimage
                .extension()
                .is_some_and(|extensao| extensao.eq_ignore_ascii_case("AppImage"));
        if !e_appimage {
            bail!("APPIMAGE não aponta para um arquivo .AppImage válido");
        }
        fs::remove_file(&appimage)
            .with_context(|| format!("removendo o AppImage {}", appimage.display()))?;
        return Ok(());
    }

    let pkexec = programa_sistema("pkexec")
        .context("não encontrei o pkexec para pedir autorização administrativa")?;
    let (gerenciador, argumentos) = comando_do_gerenciador().context(
        "não reconheci apt, dnf, pacman ou zypper; remova o pacote fol-discord manualmente",
    )?;
    let estado = Command::new(pkexec)
        .arg(&gerenciador)
        .args(argumentos)
        .status()
        .with_context(|| format!("abrindo {}", gerenciador.display()))?;
    if !estado.success() {
        bail!("o gerenciador de pacotes não removeu o fol-discord");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_aponta_para_launcher_gerenciado() {
        let texto = desktop_discord(Path::new("/home/Ana Teste/fol-discord"));
        assert!(texto.contains("Exec=\"/home/Ana Teste/fol-discord\" abrir-discord %U"));
        assert!(texto.contains(ASSINATURA));
        assert!(texto.contains("x-scheme-handler/discord"));
    }

    #[test]
    fn percentual_literal_e_escapado() {
        assert_eq!(escapar_exec("/tmp/100%/app"), "\"/tmp/100%%/app\"");
    }

    #[test]
    fn autostart_xdg_sobe_o_nucleo_sem_terminal() {
        let texto = desktop_autostart(Path::new("/opt/FOL discord/fol-discord"));
        assert!(texto.contains("Exec=\"/opt/FOL discord/fol-discord\" rodar"));
        assert!(texto.contains("Terminal=false"));
        assert!(texto.contains(ASSINATURA));
    }

    #[test]
    fn reconhece_familia_da_distribuicao() {
        let texto = "NAME=Exemplo\nID=fedora\nID_LIKE=\"rhel centos\"\nVERSION_ID=42\n";
        assert_eq!(familia_em(texto), "fedora rhel centos");
    }

    #[test]
    fn escolhe_o_gerenciador_das_quatro_familias() {
        assert_eq!(candidatos_da_familia("debian ubuntu")[0].0, "apt-get");
        assert_eq!(candidatos_da_familia("fedora rhel")[0].0, "dnf");
        assert_eq!(candidatos_da_familia("arch")[0].0, "pacman");
        assert_eq!(candidatos_da_familia("opensuse suse")[0].0, "zypper");
    }
}

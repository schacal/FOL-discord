//! Estado e comandos que mudam entre Windows e Linux.

use std::path::{Path, PathBuf};

#[cfg(windows)]
mod imp {
    use super::*;
    use winreg::{enums::*, RegKey};

    pub const NOME_SERVICO: &str = "fol-discord.exe";
    const CHAVE_INTERNET: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

    pub fn pasta_dados() -> PathBuf {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
        PathBuf::from(base).join("FolDiscord")
    }

    pub fn executavel_instalado() -> PathBuf {
        pasta_dados().join(NOME_SERVICO)
    }

    fn hkcu() -> RegKey {
        RegKey::predef(HKEY_CURRENT_USER)
    }

    pub fn pac_ativo(url: &str) -> bool {
        hkcu()
            .open_subkey(CHAVE_INTERNET)
            .and_then(|chave| chave.get_value::<String, _>("AutoConfigURL"))
            .map(|atual| atual.eq_ignore_ascii_case(url))
            .unwrap_or(false)
    }

    pub fn alterar_pac(_servico: &Path, url: &str, ligado: bool) -> Result<(), String> {
        let (chave, _) = hkcu()
            .create_subkey(CHAVE_INTERNET)
            .map_err(|erro| format!("não consegui abrir as configurações de proxy: {erro}"))?;
        if ligado {
            chave
                .set_value("AutoConfigURL", &url.to_string())
                .map_err(|erro| format!("não consegui retomar a correção: {erro}"))
        } else {
            match chave.delete_value("AutoConfigURL") {
                Ok(()) => Ok(()),
                Err(erro) if erro.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(erro) => Err(format!("não consegui pausar a correção: {erro}")),
            }
        }
    }

    pub fn preparar_executavel(_caminho: &Path) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt, process::Command};

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

    pub fn executavel_instalado() -> PathBuf {
        xdg_dados().join("fol-discord").join(NOME_SERVICO)
    }

    fn marcador_pac() -> PathBuf {
        xdg_configuracao().join("fol-discord").join("pac-url")
    }

    fn launcher_discord() -> PathBuf {
        xdg_dados()
            .join("applications")
            .join("fol-discord-discord.desktop")
    }

    fn gerenciado(caminho: &Path) -> bool {
        fs::read_to_string(caminho)
            .map(|texto| texto.lines().any(|linha| linha == ASSINATURA))
            .unwrap_or(false)
    }

    pub fn pac_ativo(url: &str) -> bool {
        gerenciado(&launcher_discord())
            && fs::read_to_string(marcador_pac())
                .map(|valor| valor.trim() == url)
                .unwrap_or(false)
    }

    fn executar(servico: &Path, comando: &str) -> Result<(), String> {
        let saida = Command::new(servico)
            .arg(comando)
            .output()
            .map_err(|erro| format!("não consegui executar {comando}: {erro}"))?;
        if saida.status.success() {
            return Ok(());
        }
        let erro = String::from_utf8_lossy(&saida.stderr);
        Err(format!("não consegui executar {comando}: {}", erro.trim()))
    }

    pub fn alterar_pac(servico: &Path, _url: &str, ligado: bool) -> Result<(), String> {
        executar(servico, if ligado { "retomar" } else { "pausar" })
    }

    pub fn preparar_executavel(caminho: &Path) -> Result<(), String> {
        let mut permissoes = fs::metadata(caminho)
            .map_err(|erro| format!("não consegui ler as permissões: {erro}"))?
            .permissions();
        permissoes.set_mode(permissoes.mode() | 0o700);
        fs::set_permissions(caminho, permissoes)
            .map_err(|erro| format!("não consegui tornar o serviço executável: {erro}"))
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
compile_error!("FOL-discord suporta somente Windows e Linux");

pub use imp::*;

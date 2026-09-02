//! Localiza, encerra e abre o Discord em cada plataforma suportada.

use anyhow::Result;
use std::{collections::BTreeSet, ffi::OsStr, process::Command, time::Duration};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

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

#[cfg(windows)]
mod imp {
    use super::*;
    use std::path::PathBuf;

    pub const NOMES: &[&str] = &["Discord.exe"];

    fn lancador() -> Option<PathBuf> {
        let base = std::env::var("LOCALAPPDATA").ok()?;
        let caminho = PathBuf::from(base).join("Discord").join("Update.exe");
        caminho.exists().then_some(caminho)
    }

    pub fn abrir(_argumentos: &[String]) -> Result<bool> {
        let Some(lancador) = lancador() else {
            return Ok(false);
        };
        comando_oculto(lancador)
            .args(["--processStart", "Discord.exe"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        Ok(true)
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::{
        env,
        path::{Path, PathBuf},
        process::Stdio,
    };

    pub const NOMES: &[&str] = &["Discord", "discord", "DiscordCanary", "DiscordPTB"];
    const FLATPAKS: &[&str] = &["com.discordapp.Discord", "com.discordapp.DiscordCanary"];

    enum Lancador {
        Nativo(PathBuf),
        Flatpak { programa: PathBuf, id: &'static str },
    }

    fn no_path(nome: &str) -> Option<PathBuf> {
        let caminho = Path::new(nome);
        if caminho.components().count() > 1 {
            return caminho.is_file().then(|| caminho.to_path_buf());
        }
        env::var_os("PATH")
            .into_iter()
            .flat_map(|valor| env::split_paths(&valor).collect::<Vec<_>>())
            .map(|pasta| pasta.join(nome))
            .find(|candidato| candidato.is_file())
    }

    fn flatpak_instalado(programa: &Path, id: &str) -> bool {
        Command::new(programa)
            .args(["info", id])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|estado| estado.success())
    }

    fn lancador() -> Option<Lancador> {
        if let Some(definido) = env::var_os("FOL_DISCORD_EXECUTAVEL_DISCORD") {
            let caminho = PathBuf::from(definido);
            if caminho.is_file() {
                return Some(Lancador::Nativo(caminho));
            }
        }

        for nome in ["discord", "discord-ptb", "discord-canary"] {
            if let Some(programa) = no_path(nome) {
                return Some(Lancador::Nativo(programa));
            }
        }

        let flatpak = no_path("flatpak")?;
        FLATPAKS
            .iter()
            .find(|id| flatpak_instalado(&flatpak, id))
            .map(|id| Lancador::Flatpak {
                programa: flatpak,
                id,
            })
    }

    pub fn abrir(argumentos: &[String]) -> Result<bool> {
        let Some(lancador) = lancador() else {
            return Ok(false);
        };
        let mut comando = match lancador {
            Lancador::Nativo(programa) => comando_oculto(programa),
            Lancador::Flatpak { programa, id } => {
                let mut comando = comando_oculto(programa);
                comando.args(["run", id]);
                comando
            }
        };
        if crate::plataforma::pac_ativo(&crate::url_pac()) {
            comando.arg(format!("--proxy-pac-url={}", crate::url_pac()));
        }
        comando
            .args(argumentos)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(true)
    }
}

pub fn pids() -> Vec<u32> {
    let mut pids = BTreeSet::new();
    for nome in imp::NOMES {
        pids.extend(crate::processos::pids_por_nome(nome));
    }
    pids.into_iter().collect()
}

pub fn esta_rodando() -> bool {
    !pids().is_empty()
}

fn encerrar() {
    crate::processos::encerrar_todos(&pids());
    std::thread::sleep(Duration::from_millis(500));
}

pub fn abrir(argumentos: &[String]) -> Result<bool> {
    imp::abrir(argumentos)
}

pub fn reiniciar() -> Result<bool> {
    let estava_aberto = esta_rodando();
    if estava_aberto {
        encerrar();
    }
    abrir(&[])
}

pub fn encerrar_se_aberto() -> bool {
    if esta_rodando() {
        encerrar();
        true
    } else {
        false
    }
}

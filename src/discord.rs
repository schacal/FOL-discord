//! Localiza e reinicia o Discord.
//!
//! A correção só vale a partir da próxima abertura do Discord. Pedir isso ao
//! usuário é um passo que ele esquece — então o instalador faz sozinho.

use anyhow::Result;
use std::{ffi::OsStr, path::PathBuf, process::Command, time::Duration};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// `tasklist`, `taskkill` e o lançador do Discord são detalhes internos: a
/// janela do FOL-discord nunca deve revelar esses processos ao usuário.
fn comando_oculto(programa: impl AsRef<OsStr>) -> Command {
    let mut comando = Command::new(programa);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        comando.creation_flags(CREATE_NO_WINDOW);
    }
    comando
}

/// O lançador do Squirrel, que sempre aponta para a versão instalada no
/// momento. Chamar por ele evita fixar `app-1.0.xxxx` no código e sobreviver
/// mal à próxima atualização do Discord.
pub fn lancador() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    let p = PathBuf::from(base).join("Discord").join("Update.exe");
    p.exists().then_some(p)
}

pub fn esta_rodando() -> bool {
    comando_oculto("tasklist")
        .args(["/FI", "IMAGENAME eq Discord.exe", "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("Discord.exe"))
        .unwrap_or(false)
}

fn encerrar() {
    let _ = comando_oculto("taskkill")
        .args(["/F", "/IM", "Discord.exe"])
        .output();
    std::thread::sleep(Duration::from_secs(3));
}

/// Fecha e reabre o Discord. Devolve `false` quando não há Discord instalado
/// — o que não é erro: o serviço fica de pé e corrige na primeira abertura.
pub fn reiniciar() -> Result<bool> {
    let Some(lancador) = lancador() else {
        return Ok(false);
    };
    let estava_aberto = esta_rodando();
    if estava_aberto {
        encerrar();
    }
    comando_oculto(lancador)
        .args(["--processStart", "Discord.exe"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(true)
}

/// Só encerra, sem reabrir. Usado na desinstalação: reabrir na hora faria o
/// Discord fixar de novo a região errada.
pub fn encerrar_se_aberto() -> bool {
    if esta_rodando() {
        encerrar();
        true
    } else {
        false
    }
}

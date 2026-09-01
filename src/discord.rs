//! Localiza e reinicia o Discord.
//!
//! A correção só vale a partir da próxima abertura do Discord. Pedir isso ao
//! usuário é um passo que ele esquece — então o instalador faz sozinho.

use anyhow::Result;
use std::{path::PathBuf, time::Duration};

/// O lançador do Squirrel, que sempre aponta para a versão instalada no
/// momento. Chamar por ele evita fixar `app-1.0.xxxx` no código e sobreviver
/// mal à próxima atualização do Discord.
pub fn lancador() -> Option<PathBuf> {
    let base = std::env::var("LOCALAPPDATA").ok()?;
    let p = PathBuf::from(base).join("Discord").join("Update.exe");
    p.exists().then_some(p)
}

pub fn esta_rodando() -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq Discord.exe", "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("Discord.exe"))
        .unwrap_or(false)
}

fn encerrar() {
    let _ = std::process::Command::new("taskkill")
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
    std::process::Command::new(lancador)
        .args(["--processStart", "Discord.exe"])
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

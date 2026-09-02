//! Localiza e reinicia o Discord.
//!
//! A correção só vale a partir da próxima abertura do Discord. Pedir isso ao
//! usuário é um passo que ele esquece — então o instalador faz sozinho.

use anyhow::Result;
use std::{ffi::OsStr, path::PathBuf, process::Command, time::Duration};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// O lançador do Discord é detalhe interno: a janela do FOL-discord nunca
/// deve revelar esse processo ao usuário.
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

const IMAGEM: &str = "Discord.exe";

pub fn esta_rodando() -> bool {
    crate::processos::esta_rodando(IMAGEM)
}

/// Todos os processos do Discord no ar. São vários — o principal, a GPU, cada
/// renderizador — e é a troca do conjunto inteiro que denuncia um reinício.
pub fn pids() -> Vec<u32> {
    crate::processos::pids_por_nome(IMAGEM)
}

/// Encerra todas as janelas do Discord e só volta quando elas saíram de fato.
/// A espera é por handle de processo, não por relógio: reabrir cedo demais faz
/// o Discord fixar de novo a região errada.
fn encerrar() {
    crate::processos::encerrar_por_nome(IMAGEM);
    // Folga curta para o Squirrel soltar os arquivos antes do relançamento.
    std::thread::sleep(Duration::from_millis(500));
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

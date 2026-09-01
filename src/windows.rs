//! As duas únicas marcas que deixamos no sistema: o proxy automático e o
//! autostart. Ambas em HKCU, sem administrador, e ambas removíveis.

use anyhow::{Context, Result};
use winreg::{enums::*, RegKey};

const CHAVE_INTERNET: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
const CHAVE_RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NOME_RUN: &str = "DesbugaDiscord";
const BACKUP: &str = "AutoConfigURL_backup_DesbugaDiscord";

fn hkcu() -> RegKey {
    RegKey::predef(HKEY_CURRENT_USER)
}

/// Aponta o proxy automático do Windows para o nosso PAC, guardando o valor
/// anterior para conseguir devolver na desinstalação.
pub fn ativar_pac(url: &str) -> Result<()> {
    let (k, _) = hkcu()
        .create_subkey(CHAVE_INTERNET)
        .context("abrindo Internet Settings")?;

    let atual: Result<String, _> = k.get_value("AutoConfigURL");
    if let Ok(anterior) = atual {
        if anterior != url && k.get_value::<String, _>(BACKUP).is_err() {
            k.set_value(BACKUP, &anterior)?;
        }
    }
    k.set_value("AutoConfigURL", &url.to_string())?;
    Ok(())
}

pub fn desativar_pac() -> Result<()> {
    let (k, _) = hkcu().create_subkey(CHAVE_INTERNET)?;
    match k.get_value::<String, _>(BACKUP) {
        Ok(anterior) => {
            k.set_value("AutoConfigURL", &anterior)?;
            let _ = k.delete_value(BACKUP);
        }
        Err(_) => {
            let _ = k.delete_value("AutoConfigURL");
        }
    }
    Ok(())
}

pub fn pac_ativo(url: &str) -> bool {
    hkcu()
        .open_subkey(CHAVE_INTERNET)
        .and_then(|k| k.get_value::<String, _>("AutoConfigURL"))
        .map(|v| v == url)
        .unwrap_or(false)
}

pub fn ativar_autostart(comando: &str) -> Result<()> {
    let (k, _) = hkcu().create_subkey(CHAVE_RUN)?;
    k.set_value(NOME_RUN, &comando.to_string())?;
    Ok(())
}

pub fn desativar_autostart() -> Result<()> {
    if let Ok(k) = hkcu().open_subkey_with_flags(CHAVE_RUN, KEY_ALL_ACCESS) {
        let _ = k.delete_value(NOME_RUN);
    }
    Ok(())
}

pub fn autostart_ativo() -> bool {
    hkcu()
        .open_subkey(CHAVE_RUN)
        .and_then(|k| k.get_value::<String, _>(NOME_RUN))
        .is_ok()
}

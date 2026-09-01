//! Ponte nativa entre a janela e o serviço estável de linha de comando.
//!
//! A primeira versão da interface tentava falar com uma API HTTP planejada na
//! porta 9252. O serviço publicado não expõe essa API: ele só usa 9250 (SOCKS)
//! e 9251 (PAC). Esta ponte usa os comandos que já funcionam, e leva uma cópia
//! deles embutida no executável da janela para que a primeira abertura consiga
//! instalar tudo sozinha.

use crate::inicializacao;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use winreg::{enums::*, RegKey};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const PORTA_PAC: u16 = 9251;
const CHAVE_INTERNET: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";

static ERRO_INICIALIZACAO: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Copiado pelo build.rs depois de compilar o serviço raiz deste repositório.
const SERVICO_EMBUTIDO: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/fol-discord.exe"));

#[derive(Clone, Serialize)]
pub struct ProxyEmUso {
    endereco: String,
    regiao: String,
    pais: String,
    latencia_ms: u64,
}

#[derive(Serialize)]
pub struct Status {
    versao: String,
    estado: &'static str,
    autostart: bool,
    pac_ligado: bool,
    proxies_saudaveis: u32,
    proxy_em_uso: Option<ProxyEmUso>,
    ultima_validacao_utc: Option<String>,
    atualizacao: Option<()>,
    erro_inicializacao: Option<String>,
}

#[derive(Serialize)]
pub struct Conexao {
    hora_utc: u64,
    host: String,
    porta: u16,
    rota: &'static str,
}

#[derive(Serialize)]
pub struct Verificacao {
    ok: bool,
    regiao_detectada: Option<String>,
    proxies_saudaveis: u32,
    mensagem: String,
}

fn pasta_dados() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("FolDiscord")
}

pub fn executavel_instalado() -> PathBuf {
    pasta_dados().join("fol-discord.exe")
}

fn caminho_log() -> PathBuf {
    pasta_dados().join("fol.log")
}

fn caminho_pronto() -> PathBuf {
    pasta_dados().join("pronto")
}

fn url_pac() -> String {
    format!("http://127.0.0.1:{PORTA_PAC}/proxy.pac")
}

/// Não abrimos uma conexão SOCKS só para ver se o processo existe: o serviço
/// registra esse teste como `early eof`, poluindo o histórico da atividade.
fn servico_rodando() -> bool {
    let mut comando = comando_oculto("tasklist");
    comando
        .args(["/FI", "IMAGENAME eq fol-discord.exe", "/NH"])
        .output()
        .map(|saida| {
            String::from_utf8_lossy(&saida.stdout)
                .to_ascii_lowercase()
                .contains("fol-discord.exe")
        })
        .unwrap_or(false)
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

fn encerrar_copias_antigas() {
    let _ = comando_oculto("taskkill")
        .args(["/F", "/IM", "fol-discord.exe"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

fn gravar_servico_embutido(destino: &Path) -> Result<(), String> {
    let pasta = destino
        .parent()
        .ok_or_else(|| "não encontrei a pasta de instalação".to_string())?;
    fs::create_dir_all(pasta).map_err(|e| format!("não consegui criar a pasta: {e}"))?;

    let novo = destino.with_extension("novo");
    fs::write(&novo, SERVICO_EMBUTIDO)
        .map_err(|e| format!("não consegui preparar o serviço: {e}"))?;
    if destino.exists() {
        fs::remove_file(destino).map_err(|e| format!("não consegui substituir o serviço antigo: {e}"))?;
    }
    fs::rename(&novo, destino).map_err(|e| format!("não consegui instalar o serviço: {e}"))
}

/// Garante que a cópia estável esteja instalada e inicia a instalação sem
/// bloquear a janela enquanto a piscina de proxies é validada.
pub fn garantir_servico(reiniciar_discord: bool, criar_run_legado: bool) -> Result<(), String> {
    let destino = executavel_instalado();
    if destino.exists() && servico_rodando() {
        return Ok(());
    }

    if destino.exists() {
        // Uma cópia sem a porta SOCKS não serve como serviço. Encerramos só o
        // executável conhecido antes de trocar o arquivo, nunca o Discord.
        encerrar_copias_antigas();
    }
    gravar_servico_embutido(&destino)?;

    let mut comando = comando_oculto(&destino);
    comando
        .arg("instalar")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if !reiniciar_discord {
        comando.arg("--sem-reiniciar");
    }
    if !criar_run_legado {
        comando.arg("--sem-autostart");
    }
    comando
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("não consegui iniciar a instalação: {e}"))
}

fn pac_ativo() -> bool {
    hkcu()
        .open_subkey(CHAVE_INTERNET)
        .and_then(|chave| chave.get_value::<String, _>("AutoConfigURL"))
        .map(|url| url.eq_ignore_ascii_case(&url_pac()))
        .unwrap_or(false)
}

fn dados_da_piscina() -> (u32, Option<ProxyEmUso>) {
    let Ok(log) = fs::read_to_string(caminho_log()) else {
        return (0, None);
    };

    let mut quantidade = 0;
    let mut proxy = None;
    for linha in log.lines().rev() {
        let texto = linha.trim();
        if quantidade == 0 {
            if let Some((numero, _)) = texto.split_once(" proxies estrangeiros validados") {
                quantidade = numero.trim().parse().unwrap_or(0);
            }
        }
        if proxy.is_none() {
            let Some((endereco, resto)) = texto.split_once(" (") else {
                continue;
            };
            let Some((regiao, latencia)) = resto.split_once(") ") else {
                continue;
            };
            let Some(ms) = latencia.strip_suffix("ms") else {
                continue;
            };
            let Ok(latencia_ms) = ms.trim().parse() else {
                continue;
            };
            proxy = Some(ProxyEmUso {
                endereco: endereco.to_string(),
                regiao: regiao.to_string(),
                pais: String::new(),
                latencia_ms,
            });
        }
    }
    (quantidade, proxy)
}

pub fn status() -> Status {
    // O comando `status` do serviço testa a porta SOCKS com uma conexão curta.
    // Isso é seguro para a linha de comando, mas a janela o chama com
    // frequência e cada teste vira uma falsa entrada `early eof` no fol.log.
    // Aqui consultamos os mesmos estados diretamente, sem tocar no SOCKS.
    let instalado = executavel_instalado().exists();
    let rodando = instalado && servico_rodando();
    let autostart = std::env::current_exe()
        .ok()
        .is_some_and(|interface| inicializacao::tarefa_ativa(&interface));
    let pac_ligado = pac_ativo();
    let (mut quantidade, proxy) = dados_da_piscina();
    let proxies_prontos = caminho_pronto().exists();
    if proxies_prontos && quantidade == 0 {
        quantidade = 1;
    }

    let estado = if !instalado || !rodando {
        "parado"
    } else if !pac_ligado {
        "pausado"
    } else if !proxies_prontos {
        "sem_proxies"
    } else {
        "operacional"
    };

    Status {
        versao: env!("CARGO_PKG_VERSION").to_string(),
        estado,
        autostart,
        pac_ligado,
        proxies_saudaveis: quantidade,
        proxy_em_uso: proxy,
        ultima_validacao_utc: None,
        atualizacao: None,
        erro_inicializacao: erro_inicializacao(),
    }
}

fn milissegundos(tempo: SystemTime) -> u64 {
    tempo
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// Lê apenas as linhas que representam uma rota tomada pelo serviço. As
/// demais linhas são diagnóstico interno e não são atividade do Discord.
fn interpretar_conexoes(log: &str, hora_utc: u64) -> Vec<Conexao> {
    log.lines()
        .rev()
        .filter_map(|linha| {
            let texto = linha.trim();
            let (rota, destino) = if let Some(destino) = texto.strip_prefix("exterior  ") {
                ("exterior", destino)
            } else if let Some(destino) = texto.strip_prefix("direto    ") {
                ("direto", destino)
            } else {
                return None;
            };
            let (host, porta) = destino.rsplit_once(':')?;
            let porta = porta.parse().ok()?;
            (!host.is_empty()).then(|| Conexao {
                hora_utc,
                host: host.to_string(),
                porta,
                rota,
            })
        })
        .take(20)
        .collect()
}

/// O serviço não registra um horário por linha. Usamos a última modificação
/// do arquivo, que é o instante verificável mais próximo disponível, e não um
/// horário inventado pela interface.
pub fn conexoes() -> Vec<Conexao> {
    let caminho = caminho_log();
    let Ok(log) = fs::read_to_string(&caminho) else {
        return Vec::new();
    };
    let hora_utc = fs::metadata(caminho)
        .and_then(|metadados| metadados.modified())
        .map(milissegundos)
        .unwrap_or_else(|_| milissegundos(SystemTime::now()));
    interpretar_conexoes(&log, hora_utc)
}

fn hkcu() -> RegKey {
    RegKey::predef(HKEY_CURRENT_USER)
}

pub fn pausar() -> Result<(), String> {
    if !executavel_instalado().exists() {
        return Err("o serviço ainda não está instalado".into());
    }
    let (chave, _) = hkcu()
        .create_subkey(CHAVE_INTERNET)
        .map_err(|e| format!("não consegui abrir as configurações de proxy: {e}"))?;
    match chave.delete_value("AutoConfigURL") {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("não consegui pausar a correção: {e}")),
    }
}

pub fn retomar() -> Result<(), String> {
    if !executavel_instalado().exists() {
        return Err("o serviço ainda não está instalado".into());
    }
    let (chave, _) = hkcu()
        .create_subkey(CHAVE_INTERNET)
        .map_err(|e| format!("não consegui abrir as configurações de proxy: {e}"))?;
    chave
        .set_value("AutoConfigURL", &url_pac())
        .map_err(|e| format!("não consegui retomar a correção: {e}"))
}

pub fn definir_autostart(ligado: bool) -> Result<(), String> {
    let servico = executavel_instalado();
    if !servico.exists() {
        return Err("o serviço ainda não está instalado".into());
    }
    let interface = std::env::current_exe()
        .map_err(|erro| format!("não encontrei a interface atual: {erro}"))?;
    if ligado {
        inicializacao::ativar_tarefa(&interface, &servico)
    } else {
        inicializacao::desativar_tarefa()
    }
}

pub fn verificar() -> Result<Verificacao, String> {
    garantir_servico(false, false)?;
    for _ in 0..10 {
        let atual = status();
        if atual.estado != "parado" {
            return Ok(verificacao_do_status(&atual));
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Ok(Verificacao {
        ok: false,
        regiao_detectada: None,
        proxies_saudaveis: 0,
        mensagem: "O serviço ainda está iniciando. Aguarde alguns segundos e verifique de novo.".into(),
    })
}

fn verificacao_do_status(atual: &Status) -> Verificacao {
    let regiao = atual.proxy_em_uso.as_ref().map(|p| p.regiao.clone());
    let (ok, mensagem) = match atual.estado {
        "operacional" => (true, "O serviço e os proxies estão prontos."),
        "pausado" => (false, "A correção está pausada."),
        "sem_proxies" => (false, "O serviço está ligado, mas ainda procura proxies saudáveis."),
        _ => (false, "O serviço não está respondendo."),
    };
    Verificacao {
        ok,
        regiao_detectada: regiao,
        proxies_saudaveis: atual.proxies_saudaveis,
        mensagem: mensagem.into(),
    }
}

pub fn reiniciar_discord() -> Result<bool, String> {
    let executavel = executavel_instalado();
    if !executavel.exists() || !servico_rodando() {
        return Err("o serviço precisa estar rodando antes de reiniciar o Discord".into());
    }

    let saida = comando_oculto(&executavel)
        .arg("reiniciar-discord")
        .output()
        .map_err(|e| format!("não consegui reiniciar o Discord: {e}"))?;
    if !saida.status.success() {
        let erro = String::from_utf8_lossy(&saida.stderr);
        return Err(format!("não consegui reiniciar o Discord: {}", erro.trim()));
    }
    Ok(String::from_utf8_lossy(&saida.stdout).contains("Discord reiniciado"))
}

pub fn comando_desinstalador() -> Result<Command, String> {
    inicializacao::comando_desinstalador()
}

pub fn registrar_erro_inicializacao(erro: String) {
    let erros = ERRO_INICIALIZACAO.get_or_init(|| Mutex::new(None));
    if let Ok(mut atual) = erros.lock() {
        *atual = Some(erro);
    }
}

fn erro_inicializacao() -> Option<String> {
    ERRO_INICIALIZACAO
        .get()
        .and_then(|erros| erros.lock().ok().and_then(|erro| erro.clone()))
}

#[cfg(test)]
mod tests {
    use super::interpretar_conexoes;

    #[test]
    fn extrai_somente_as_rotas_reais_do_log() {
        let conexoes = interpretar_conexoes(
            "reabastecendo a piscina\n\
exterior  gateway.discord.gg:443\n\
direto    cdn.discordapp.com:443\n\
conexão encerrada: early eof\n",
            1_700_000_000_000,
        );

        assert_eq!(conexoes.len(), 2);
        assert_eq!(conexoes[0].host, "cdn.discordapp.com");
        assert_eq!(conexoes[0].rota, "direto");
        assert_eq!(conexoes[1].host, "gateway.discord.gg");
        assert_eq!(conexoes[1].rota, "exterior");
    }
}

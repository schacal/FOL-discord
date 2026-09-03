//! Localiza, encerra e abre o Discord em cada plataforma suportada.

use anyhow::Result;
use std::{collections::BTreeSet, ffi::OsStr, process::Command, time::Duration};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use crate::{processos::Processo, sessao::Identidade};

/// O lançador do Discord é detalhe interno: a janela do FOL-discord nunca
/// deve revelar esse processo ao usuário.
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

/// O processo principal do Discord no ar, com a hora em que nasceu.
///
/// O Discord roda vários processos — numa máquina comum são o principal e
/// vários filhos, entre GPU, renderizadores e utilitários. Filho nasce e morre
/// no meio do uso; o principal, não. É a identidade dele que denuncia um
/// reinício, e é por isso que um Ctrl+R não conta como um.
///
/// O principal é o único cujo pai não é outro processo do Discord. A hora de
/// criação vai junto porque o sistema pode reaproveitar o PID do processo
/// antigo para um Discord novo.
pub fn principal() -> Option<Identidade> {
    let processos: Vec<Processo> = imp::NOMES
        .iter()
        .flat_map(|nome| crate::processos::processos_por_nome(nome))
        .collect();
    principal_entre(&processos, crate::processos::criado_em)
}

fn principal_entre(
    processos: &[Processo],
    criado_em: impl Fn(u32) -> Option<u64>,
) -> Option<Identidade> {
    let nascidos: Vec<(Processo, Option<u64>)> =
        processos.iter().map(|p| (*p, criado_em(p.pid))).collect();

    // Um pai de verdade nasceu antes do filho. O lançador morre logo depois
    // de criar o principal, e o PID dele pode ser reaproveitado por um filho
    // do próprio Discord — aí o principal pareceria ter pai Discord, ninguém
    // sobraria como raiz, e o vigia declararia o Discord fechado com ele
    // aberto. Sem hora de criação de um dos dois, vale só o PID.
    let e_pai_de_verdade = |pai: u32, filho_nasceu: Option<u64>| {
        nascidos.iter().any(|(q, q_nasceu)| {
            q.pid == pai
                && match (q_nasceu, filho_nasceu) {
                    (Some(pai_nasceu), Some(filho_nasceu)) => *pai_nasceu <= filho_nasceu,
                    _ => true,
                }
        })
    };

    // Entre candidatos, o mais antigo — e quem tem hora conhecida vence quem
    // não tem, para um processo que o Windows não deixa consultar nunca
    // passar na frente do Discord deste usuário.
    let mais_antigo =
        |(p, nasceu): &&(Processo, Option<u64>)| (nasceu.is_none(), nasceu.unwrap_or(0), p.pid);

    let raiz = nascidos
        .iter()
        .filter(|(p, nasceu)| !e_pai_de_verdade(p.pai, *nasceu))
        .min_by_key(mais_antigo)
        // Com Discord na lista sempre há um Discord: se a árvore ficou sem
        // raiz por um pai fantasma, o mais antigo de todos é o principal.
        .or_else(|| nascidos.iter().min_by_key(mais_antigo))?;

    Some(Identidade {
        pid: raiz.0.pid,
        criado_em: raiz.1.unwrap_or(0),
    })
}

/// Encerra todas as janelas do Discord e só volta quando elas saíram de fato.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, pai: u32) -> Processo {
        Processo { pid, pai }
    }

    /// A árvore de uma máquina de verdade: o principal nasceu do lançador
    /// (que já saiu) e os seis filhos nasceram dele.
    fn arvore_comum() -> Vec<Processo> {
        vec![
            p(4000, 3990),
            p(4100, 4000),
            p(4200, 4000),
            p(4300, 4000),
            p(4400, 4000),
            p(4500, 4000),
            p(4600, 4000),
        ]
    }

    #[test]
    fn o_principal_e_quem_nao_tem_pai_discord() {
        let principal = principal_entre(&arvore_comum(), |pid| Some(u64::from(pid) * 10));
        assert_eq!(
            principal,
            Some(Identidade {
                pid: 4000,
                criado_em: 40_000
            })
        );
    }

    #[test]
    fn filho_novo_nao_muda_o_principal() {
        let mut arvore = arvore_comum();
        let antes = principal_entre(&arvore, |_| Some(1));

        // Um renderizador morreu e outro nasceu: Ctrl+R, troca de servidor.
        arvore.retain(|p| p.pid != 4300);
        arvore.push(p(4700, 4000));
        assert_eq!(principal_entre(&arvore, |_| Some(1)), antes);
    }

    #[test]
    fn sem_discord_nao_ha_principal() {
        assert_eq!(principal_entre(&[], |_| Some(1)), None);
    }

    #[test]
    fn sem_hora_de_criacao_o_pid_ainda_identifica() {
        // O Windows recusou a pergunta — processo de outro usuário, por
        // exemplo. Melhor um Discord identificado só pelo PID do que nenhum.
        let principal = principal_entre(&arvore_comum(), |_| None);
        assert_eq!(principal.map(|i| i.pid), Some(4000));
    }

    #[test]
    fn pid_do_lancador_reaproveitado_por_um_filho_nao_esconde_o_principal() {
        // O principal nasceu do lançador 3990, que morreu; um renderizador
        // nasceu depois com esse mesmo número. Pelo PID sozinho a árvore vira
        // um ciclo sem raiz — e o vigia declararia o Discord fechado com ele
        // aberto, deixando a janela em Abertura sem prazo nenhum.
        let arvore = vec![p(4000, 3990), p(3990, 4000), p(4200, 4000), p(4300, 4000)];
        let hora = |pid: u32| {
            Some(if pid == 4000 {
                100
            } else {
                105 + u64::from(pid)
            })
        };
        assert_eq!(
            principal_entre(&arvore, hora),
            Some(Identidade {
                pid: 4000,
                criado_em: 100
            })
        );

        // Sem hora nenhuma o ciclo não tem como ser desfeito, mas a lista não
        // está vazia: ainda assim há um Discord, e a escolha é estável.
        let sem_hora = principal_entre(&arvore, |_| None);
        assert!(sem_hora.is_some(), "Discord de pé nunca vira None");
        assert_eq!(principal_entre(&arvore, |_| None), sem_hora);
    }

    #[test]
    fn raiz_sem_hora_nao_passa_na_frente_da_raiz_com_hora() {
        // Duas raízes: a deste usuário, com hora, e uma que o Windows não
        // deixa consultar. A desconhecida não pode virar a identidade.
        let arvore = vec![p(4000, 1), p(9000, 2)];
        let hora = |pid: u32| (pid == 4000).then_some(500);
        assert_eq!(principal_entre(&arvore, hora).map(|i| i.pid), Some(4000));
    }

    #[test]
    fn com_o_principal_morto_os_filhos_ainda_dao_uma_identidade_estavel() {
        // O principal caiu e os filhos ainda não: todos viram raiz. A escolha
        // é a mais antiga, e continua a mesma enquanto eles não morrerem.
        let orfaos = vec![p(4100, 4000), p(4200, 4000), p(4300, 4000)];
        let hora = |pid: u32| Some(u64::from(pid));
        assert_eq!(principal_entre(&orfaos, hora).map(|i| i.pid), Some(4100));
        assert_eq!(principal_entre(&orfaos, hora).map(|i| i.pid), Some(4100));
    }
}

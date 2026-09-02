//! Encontra e encerra processos pela API do Windows, sem chamar utilitário externo.
//!
//! O caminho óbvio seria `tasklist` e `taskkill`. Os dois funcionam, e os dois
//! são exatamente o que um antivírus observa numa detonação: um processo sem
//! janela que enumera a lista de processos e mata um programa de terceiros por
//! força bruta. Não é o que este programa faz — ele fecha e reabre o Discord,
//! e encerra cópias antigas de si mesmo — mas é o que a heurística vê.
//!
//! Falar direto com a API custa o mesmo, tira dois binários do sistema da
//! árvore de processos e ainda corrige um defeito real: o `tasklist` era lido
//! pela saída em texto, que muda com o idioma do Windows.

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        Storage::FileSystem::SYNCHRONIZE,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_TERMINATE},
        },
    };

    /// Quanto esperar cada processo morrer de fato. O `taskkill` que estava
    /// aqui antes não esperava nada — quem chamava dormia três segundos e
    /// torcia. Esperar pelo handle acerta sempre e costuma voltar bem antes.
    const ESPERA_MS: u32 = 5_000;

    /// Todos os PIDs cujo nome de imagem casa com `nome`, sem diferenciar
    /// maiúsculas — é assim que o Windows compara nome de executável.
    pub fn pids_por_nome(nome: &str) -> Vec<u32> {
        let procurado: Vec<u16> = std::ffi::OsStr::new(nome).encode_wide().collect();
        let mut achados = Vec::new();

        // SAFETY: o snapshot é fechado em todos os caminhos de saída, e a
        // entrada tem `dwSize` preenchido como a API exige.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return achados;
            }

            let mut entrada: PROCESSENTRY32W = std::mem::zeroed();
            entrada.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snapshot, &mut entrada) != 0 {
                loop {
                    if mesmo_nome(&entrada.szExeFile, &procurado) {
                        achados.push(entrada.th32ProcessID);
                    }
                    if Process32NextW(snapshot, &mut entrada) == 0 {
                        break;
                    }
                }
            }

            CloseHandle(snapshot);
        }

        achados
    }

    /// Encerra cada PID e espera ele sair. Falha individual é silenciosa de
    /// propósito: o processo pode ter morrido sozinho no meio do caminho, ou
    /// pertencer a outra sessão — nenhum dos dois é motivo para abortar o resto.
    pub fn encerrar_todos(pids: &[u32]) {
        for pid in pids {
            // SAFETY: o handle é fechado logo abaixo, e só é usado quando a
            // abertura devolveu algo não nulo.
            unsafe {
                let processo = OpenProcess(PROCESS_TERMINATE | SYNCHRONIZE, 0, *pid);
                if processo.is_null() {
                    continue;
                }
                TerminateProcess(processo, 1);
                WaitForSingleObject(processo, ESPERA_MS);
                CloseHandle(processo);
            }
        }
    }

    /// `szExeFile` é um buffer fixo terminado em NUL; comparar o buffer inteiro
    /// acharia lixo depois do nome.
    fn mesmo_nome(bruto: &[u16; 260], procurado: &[u16]) -> bool {
        let fim = bruto.iter().position(|c| *c == 0).unwrap_or(bruto.len());
        let nome = &bruto[..fim];
        nome.len() == procurado.len()
            && nome
                .iter()
                .zip(procurado)
                .all(|(a, b)| caixa_baixa(*a) == caixa_baixa(*b))
    }

    fn caixa_baixa(c: u16) -> u16 {
        match u8::try_from(c) {
            Ok(b) => u16::from(b.to_ascii_lowercase()),
            Err(_) => c,
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::{
        fs,
        path::Path,
        time::{Duration, Instant},
    };

    const ESPERA: Duration = Duration::from_secs(5);

    fn uid(caminho: &Path) -> Option<u32> {
        fs::read_to_string(caminho.join("status"))
            .ok()?
            .lines()
            .find_map(|linha| linha.strip_prefix("Uid:"))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    }

    fn nome(caminho: &Path) -> Option<String> {
        fs::read_to_string(caminho.join("comm"))
            .ok()
            .map(|valor| valor.trim().to_string())
            .filter(|valor| !valor.is_empty())
            .or_else(|| {
                fs::read_link(caminho.join("exe"))
                    .ok()?
                    .file_name()
                    .map(|valor| valor.to_string_lossy().into_owned())
            })
    }

    pub fn pids_por_nome(procurado: &str) -> Vec<u32> {
        let meu_uid = uid(Path::new("/proc/self"));
        let Ok(entradas) = fs::read_dir("/proc") else {
            return Vec::new();
        };
        entradas
            .flatten()
            .filter_map(|entrada| {
                let pid: u32 = entrada.file_name().to_str()?.parse().ok()?;
                let caminho = entrada.path();
                if meu_uid.is_some() && uid(&caminho) != meu_uid {
                    return None;
                }
                (nome(&caminho).as_deref() == Some(procurado)).then_some(pid)
            })
            .collect()
    }

    fn existe(pid: u32) -> bool {
        Path::new("/proc").join(pid.to_string()).exists()
    }

    pub fn encerrar_todos(pids: &[u32]) {
        for &pid in pids {
            if pid == std::process::id() {
                continue;
            }
            // SAFETY: `kill` não retém ponteiros; o PID foi descoberto em /proc
            // e o sinal é uma constante válida do libc.
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGTERM);
            }
        }

        let limite = Instant::now() + ESPERA;
        while Instant::now() < limite && pids.iter().any(|pid| existe(*pid)) {
            std::thread::sleep(Duration::from_millis(50));
        }

        for &pid in pids.iter().filter(|pid| existe(**pid)) {
            unsafe {
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn encontra_o_proprio_processo_no_proc() {
            let nome = fs::read_to_string("/proc/self/comm").unwrap();
            assert!(pids_por_nome(nome.trim()).contains(&std::process::id()));
        }
    }
}

pub use imp::{encerrar_todos, pids_por_nome};

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

/// Um processo visto na lista do Windows: quem ele é e quem o criou. O pai é
/// o PID que o Windows anotou na criação; se esse pai já morreu, o número
/// pode ter sido reaproveitado por outro programa qualquer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Processo {
    pub pid: u32,
    pub pai: u32,
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::{
        Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE},
        Storage::FileSystem::SYNCHRONIZE,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                GetProcessTimes, OpenProcess, TerminateProcess, WaitForSingleObject,
                PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
            },
        },
    };

    use super::Processo;

    /// Quanto esperar cada processo morrer de fato. O `taskkill` que estava
    /// aqui antes não esperava nada — quem chamava dormia três segundos e
    /// torcia. Esperar pelo handle acerta sempre e costuma voltar bem antes.
    const ESPERA_MS: u32 = 5_000;

    /// Todos os processos cujo nome de imagem casa com `nome`, sem diferenciar
    /// maiúsculas — é assim que o Windows compara nome de executável.
    ///
    /// Volta vazio quando o retrato da lista falha, o que acontece de vez em
    /// quando sob carga. Quem depende de "não há nenhum" precisa tolerar isso.
    pub fn processos_por_nome(nome: &str) -> Vec<Processo> {
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
                        achados.push(Processo {
                            pid: entrada.th32ProcessID,
                            pai: entrada.th32ParentProcessID,
                        });
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

    /// Hora em que o processo nasceu, como o Windows a guarda: centenas de
    /// nanossegundos desde 1601. `None` se ele já sumiu ou não deixa
    /// perguntar — e aí quem chama se vira só com o PID.
    pub fn criado_em(pid: u32) -> Option<u64> {
        // SAFETY: o handle é fechado logo abaixo, e só é usado quando a
        // abertura devolveu algo não nulo. As quatro estruturas são
        // preenchidas pela API antes de serem lidas.
        unsafe {
            let processo = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if processo.is_null() {
                return None;
            }
            let mut criacao: FILETIME = std::mem::zeroed();
            let mut saida: FILETIME = std::mem::zeroed();
            let mut nucleo: FILETIME = std::mem::zeroed();
            let mut usuario: FILETIME = std::mem::zeroed();
            let ok = GetProcessTimes(processo, &mut criacao, &mut saida, &mut nucleo, &mut usuario);
            CloseHandle(processo);
            (ok != 0).then(|| {
                (u64::from(criacao.dwHighDateTime) << 32) | u64::from(criacao.dwLowDateTime)
            })
        }
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

#[cfg(not(windows))]
mod imp {
    use super::Processo;

    pub fn processos_por_nome(_nome: &str) -> Vec<Processo> {
        Vec::new()
    }

    pub fn criado_em(_pid: u32) -> Option<u64> {
        None
    }

    pub fn encerrar_todos(_pids: &[u32]) {}
}

pub use imp::{criado_em, encerrar_todos, processos_por_nome};

/// Só os PIDs, para quem não se importa com a árvore.
pub fn pids_por_nome(nome: &str) -> Vec<u32> {
    processos_por_nome(nome).into_iter().map(|p| p.pid).collect()
}

/// Atalho para o caso mais comum: encerrar tudo que atende por um nome.
pub fn encerrar_por_nome(nome: &str) {
    encerrar_todos(&pids_por_nome(nome));
}

pub fn esta_rodando(nome: &str) -> bool {
    !pids_por_nome(nome).is_empty()
}

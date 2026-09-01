//! Piscina de proxies SOCKS5 públicos, com auto-cura.
//!
//! Busca listas públicas, valida cada candidato contra o próprio Discord e
//! mantém uma fila ordenada por latência. Quem falha em uso é rebaixado; a
//! piscina se reabastece sozinha quando fica magra.

use anyhow::{anyhow, Result};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::task::JoinSet;

const LISTAS: &[&str] = &[
    "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/socks5.txt",
    "https://raw.githubusercontent.com/TheSpeedX/PROXY-List/master/socks5.txt",
    "https://api.proxyscrape.com/v4/free-proxy-list/get?request=display_proxies&protocol=socks5&proxy_format=ipport&format=text",
];

/// Endpoint que devolve a lista de regiões de voz conforme o IP de origem.
/// É o mesmo sinal que o Discord usa, então validar por aqui é validar de verdade.
const SONDA: &str = "https://latency.discord.media/rtc";

const ALVO_SAUDAVEIS: usize = 5;
const VALIDACOES_SIMULTANEAS: usize = 60;
const TIMEOUT_VALIDACAO: Duration = Duration::from_secs(8);

/// Todo tráfego do serviço se identifica. Requisição HTTP anônima saindo de um
/// processo sem janela é sinal barato para sandbox e feed de reputação — e é
/// exatamente o formato de quem baixa configuração de um servidor de comando.
/// Com um nome e o endereço do repositório, o mesmo tráfego passa a ser
/// atribuível a um programa que qualquer um pode ler.
const IDENTIFICACAO: &str = concat!(
    "FOL-discord/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/schacal/FOL-discord)"
);

#[derive(Clone, Debug)]
pub struct Upstream {
    pub endereco: String,
    pub latencia: Duration,
    pub regiao: String,
    falhas: u32,
}

#[derive(Clone)]
pub struct Pool {
    saudaveis: Arc<Mutex<Vec<Upstream>>>,
}

impl Pool {
    pub fn nova() -> Self {
        Self {
            saudaveis: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Melhor upstream disponível, ou `None` se a piscina está seca.
    pub fn melhor(&self) -> Option<String> {
        self.saudaveis
            .lock()
            .ok()?
            .first()
            .map(|u| u.endereco.clone())
    }

    pub fn quantidade(&self) -> usize {
        self.saudaveis.lock().map(|v| v.len()).unwrap_or(0)
    }

    pub fn listar(&self) -> Vec<Upstream> {
        self.saudaveis.lock().map(|v| v.clone()).unwrap_or_default()
    }

    /// Registra que um upstream falhou em uso. Duas falhas e ele sai da fila.
    pub fn marcar_falha(&self, endereco: &str) {
        if let Ok(mut v) = self.saudaveis.lock() {
            if let Some(u) = v.iter_mut().find(|u| u.endereco == endereco) {
                u.falhas += 1;
            }
            v.retain(|u| u.falhas < 2);
        }
    }

    fn definir(&self, mut novos: Vec<Upstream>) {
        novos.sort_by_key(|u| u.latencia);
        if let Ok(mut v) = self.saudaveis.lock() {
            *v = novos;
        }
    }

    /// Reabastece a piscina. Devolve quantos ficaram saudáveis.
    pub async fn reabastecer(&self) -> Result<usize> {
        let candidatos = baixar_listas().await?;
        if candidatos.is_empty() {
            return Err(anyhow!("nenhum candidato nas listas públicas"));
        }

        let bons = Arc::new(Mutex::new(Vec::<Upstream>::new()));
        let mut fila = JoinSet::new();
        let mut iter = candidatos.into_iter();

        for _ in 0..VALIDACOES_SIMULTANEAS {
            if let Some(c) = iter.next() {
                fila.spawn(validar(c));
            }
        }

        while let Some(res) = fila.join_next().await {
            if let Ok(Some(u)) = res {
                let mut b = bons.lock().unwrap();
                b.push(u);
                if b.len() >= ALVO_SAUDAVEIS * 3 {
                    break;
                }
            }
            if let Some(c) = iter.next() {
                fila.spawn(validar(c));
            }
        }
        fila.abort_all();

        let encontrados = Arc::try_unwrap(bons)
            .map(|m| m.into_inner().unwrap())
            .unwrap_or_default();
        let n = encontrados.len();
        self.definir(encontrados);
        Ok(n)
    }
}

async fn baixar_listas() -> Result<Vec<String>> {
    let cliente = reqwest::Client::builder()
        .user_agent(IDENTIFICACAO)
        .timeout(Duration::from_secs(30))
        .build()?;

    let mut vistos = HashSet::new();
    let mut saida = Vec::new();

    for url in LISTAS {
        let Ok(resp) = cliente.get(*url).send().await else {
            continue;
        };
        let Ok(texto) = resp.text().await else { continue };
        for linha in texto.lines() {
            let l = linha.trim();
            if parece_ip_porta(l) && vistos.insert(l.to_string()) {
                saida.push(l.to_string());
            }
        }
    }
    Ok(saida)
}

fn parece_ip_porta(s: &str) -> bool {
    let Some((ip, porta)) = s.rsplit_once(':') else {
        return false;
    };
    porta.parse::<u16>().is_ok()
        && ip.split('.').count() == 4
        && ip.split('.').all(|o| o.parse::<u8>().is_ok())
}

/// Um candidato só entra na piscina se atender três coisas ao mesmo tempo:
/// estar de pé, conseguir falar com o Discord, e **não** cair na região `brazil`.
async fn validar(endereco: String) -> Option<Upstream> {
    let cliente = reqwest::Client::builder()
        .user_agent(IDENTIFICACAO)
        .proxy(reqwest::Proxy::all(format!("socks5h://{endereco}")).ok()?)
        .timeout(TIMEOUT_VALIDACAO)
        .build()
        .ok()?;

    let t0 = Instant::now();
    let resp = cliente.get(SONDA).send().await.ok()?;
    let regioes: Vec<serde_json::Value> = resp.json().await.ok()?;
    let latencia = t0.elapsed();

    let primeira = regioes.first()?.get("region")?.as_str()?.to_string();
    if primeira == "brazil" {
        return None;
    }

    Some(Upstream {
        endereco,
        latencia,
        regiao: primeira,
        falhas: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconhece_ip_porta() {
        assert!(parece_ip_porta("5.255.99.75:1080"));
        assert!(!parece_ip_porta("exemplo.com:1080"));
        assert!(!parece_ip_porta("5.255.99.75"));
        assert!(!parece_ip_porta("5.255.99.999:1080"));
        assert!(!parece_ip_porta("5.255.99.75:99999"));
    }

    #[test]
    fn duas_falhas_removem_da_fila() {
        let p = Pool::nova();
        p.definir(vec![Upstream {
            endereco: "1.2.3.4:1080".into(),
            latencia: Duration::from_millis(100),
            regiao: "rotterdam".into(),
            falhas: 0,
        }]);
        assert_eq!(p.quantidade(), 1);
        p.marcar_falha("1.2.3.4:1080");
        assert_eq!(p.quantidade(), 1, "uma falha ainda perdoa");
        p.marcar_falha("1.2.3.4:1080");
        assert_eq!(p.quantidade(), 0, "duas falhas eliminam");
    }

    #[test]
    fn melhor_e_o_de_menor_latencia() {
        let p = Pool::nova();
        p.definir(vec![
            Upstream {
                endereco: "lento:1080".into(),
                latencia: Duration::from_millis(900),
                regiao: "frankfurt".into(),
                falhas: 0,
            },
            Upstream {
                endereco: "rapido:1080".into(),
                latencia: Duration::from_millis(120),
                regiao: "rotterdam".into(),
                falhas: 0,
            },
        ]);
        assert_eq!(p.melhor().unwrap(), "rapido:1080");
    }
}

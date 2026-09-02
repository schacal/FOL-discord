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
use tokio::{sync::Notify, task::JoinSet};

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

/// Abaixo disto a piscina está magra e a manutenção reabastece. É também o
/// ponto em que ela acorda antes da hora — ver `esperar_secar`.
pub const MINIMO_SAUDAVEIS: usize = 3;

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
    secou: Arc<Notify>,
}

impl Pool {
    pub fn nova() -> Self {
        Self {
            saudaveis: Arc::new(Mutex::new(Vec::new())),
            secou: Arc::new(Notify::new()),
        }
    }

    /// Melhor upstream disponível, ou `None` se a piscina está seca.
    #[cfg(test)]
    pub fn melhor(&self) -> Option<String> {
        self.melhor_exceto(&[])
    }

    /// Melhor upstream fora de `exceto`, ou `None` se não sobrou nenhum. É o
    /// que dá à segunda tentativa de uma conexão um proxy diferente do que
    /// acabou de falhar: `marcar_falha` só elimina na segunda falha, então sem
    /// isto a segunda volta pegava o mesmo endereço e gastava outro prazo
    /// inteiro nele — e a conexão caía para direto com a piscina cheia.
    pub fn melhor_exceto(&self, exceto: &[String]) -> Option<String> {
        self.saudaveis
            .lock()
            .ok()?
            .iter()
            .find(|u| !exceto.contains(&u.endereco))
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
        let magra = match self.saudaveis.lock() {
            Ok(mut v) => {
                if let Some(u) = v.iter_mut().find(|u| u.endereco == endereco) {
                    u.falhas += 1;
                }
                v.retain(|u| u.falhas < 2);
                v.len() < MINIMO_SAUDAVEIS
            }
            Err(_) => false,
        };
        if magra {
            self.secou.notify_one();
        }
    }

    /// Resolve quando a piscina cai abaixo do mínimo em uso. É o que acorda a
    /// manutenção antes da hora: sem isto, uma piscina que secasse logo depois
    /// de uma passada ficava seca por até cinco minutos — e a janela de
    /// abertura, que não vence sem proxy, ficava aberta esse tempo todo com
    /// o Discord caindo para direto. Com todo o Discord saindo pelo exterior
    /// na abertura, secar ficou bem mais fácil.
    ///
    /// Só volta quando a piscina está magra de fato: um aviso guardado
    /// enquanto a manutenção estava ocupada reabastecendo não conta, porque a
    /// piscina que ele descrevia já foi trocada.
    pub async fn esperar_secar(&self) {
        loop {
            self.secou.notified().await;
            if self.quantidade() < MINIMO_SAUDAVEIS {
                return;
            }
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
impl Pool {
    /// Uma piscina já cheia, na ordem dada, para os testes de quem a consome.
    pub fn de_teste(enderecos: &[&str]) -> Self {
        let p = Pool::nova();
        p.definir(
            enderecos
                .iter()
                .enumerate()
                .map(|(i, e)| Upstream {
                    endereco: e.to_string(),
                    latencia: Duration::from_millis(100 * (i as u64 + 1)),
                    regiao: "rotterdam".into(),
                    falhas: 0,
                })
                .collect(),
        );
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn upstream(endereco: &str, latencia_ms: u64) -> Upstream {
        Upstream {
            endereco: endereco.into(),
            latencia: Duration::from_millis(latencia_ms),
            regiao: "rotterdam".into(),
            falhas: 0,
        }
    }

    #[test]
    fn a_segunda_escolha_pula_quem_acabou_de_falhar() {
        let p = Pool::de_teste(&["a:1080", "b:1080"]);
        assert_eq!(p.melhor().as_deref(), Some("a:1080"));

        // Uma falha ainda perdoa o primeiro — ele continua na fila e na
        // frente. Mas quem acabou de vê-lo falhar não tem por que insistir.
        p.marcar_falha("a:1080");
        assert_eq!(p.melhor().as_deref(), Some("a:1080"));
        assert_eq!(p.melhor_exceto(&["a:1080".into()]).as_deref(), Some("b:1080"));
        assert_eq!(p.melhor_exceto(&["a:1080".into(), "b:1080".into()]), None);
    }

    #[tokio::test]
    async fn aviso_velho_nao_acorda_a_manutencao_com_a_piscina_cheia() {
        let p = Pool::de_teste(&["a:1080"]);

        // A piscina secou enquanto a manutenção estava ocupada — e depois foi
        // reposta. O aviso guardado descreve uma piscina que já não existe.
        p.marcar_falha("a:1080");
        p.definir(
            (0..MINIMO_SAUDAVEIS + 1)
                .map(|i| upstream(&format!("10.0.0.{i}:1080"), 100))
                .collect(),
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(100), p.esperar_secar())
                .await
                .is_err(),
            "com a piscina cheia não há o que reabastecer"
        );
    }

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
        p.definir(vec![upstream("1.2.3.4:1080", 100)]);
        assert_eq!(p.quantidade(), 1);
        p.marcar_falha("1.2.3.4:1080");
        assert_eq!(p.quantidade(), 1, "uma falha ainda perdoa");
        p.marcar_falha("1.2.3.4:1080");
        assert_eq!(p.quantidade(), 0, "duas falhas eliminam");
    }

    #[tokio::test]
    async fn secar_acorda_a_manutencao() {
        let p = Pool::nova();
        p.definir(vec![upstream("1.2.3.4:1080", 100)]);

        // Uma falha ainda perdoa o upstream e a fila não mudou de tamanho —
        // mas com um só ela já está abaixo do mínimo, então avisa.
        p.marcar_falha("1.2.3.4:1080");
        assert!(
            tokio::time::timeout(Duration::from_millis(200), p.esperar_secar())
                .await
                .is_ok(),
            "abaixo do mínimo a manutenção tem que acordar"
        );
    }

    #[tokio::test]
    async fn piscina_cheia_nao_acorda_a_manutencao() {
        let p = Pool::nova();
        p.definir(
            (0..MINIMO_SAUDAVEIS + 1)
                .map(|i| upstream(&format!("10.0.0.{i}:1080"), 100))
                .collect(),
        );

        p.marcar_falha("10.0.0.0:1080");
        assert!(
            tokio::time::timeout(Duration::from_millis(100), p.esperar_secar())
                .await
                .is_err(),
            "uma falha numa piscina cheia não é motivo para reabastecer"
        );
    }

    #[test]
    fn melhor_e_o_de_menor_latencia() {
        let p = Pool::nova();
        p.definir(vec![
            {
                let mut u = upstream("lento:1080", 900);
                u.regiao = "frankfurt".into();
                u
            },
            upstream("rapido:1080", 120),
        ]);
        assert_eq!(p.melhor().unwrap(), "rapido:1080");
    }
}

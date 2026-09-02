//! Em que momento da vida da sessão o Discord está.
//!
//! O IP estrangeiro só é lido uma vez, quando a sessão nasce; daí em diante a
//! região já está decidida e gravada nela. É a lógica da correção manual —
//! ligar a VPN, abrir o Discord, desligar a VPN —, e este módulo é o dedo no
//! botão: abre uma janela quando o Discord aparece, fecha quando a rajada de
//! abertura passa, e derruba o que ficou preso no exterior para o Discord
//! reconectar direto.
//!
//! O relógio que fecha a janela é alimentado só pelas conexões que decidem a
//! região (`routing::decide_regiao`). Tudo o mais do Discord também sai pelo
//! exterior enquanto ela está aberta, mas não a segura: senão um Discord em
//! uso — trocando de canal, carregando imagem — nunca deixaria o silêncio
//! completar, e a janela só fecharia pelo teto, no meio do uso.

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::sync::broadcast;

use crate::routing;

/// Silêncio que declara a rajada de abertura encerrada.
///
/// Trinta segundos vêm de cronometrar aberturas de verdade nesta máquina, em
/// duas corridas. Numa, o Discord falou com o `discord.com` aos 2,4 s, com o
/// gateway aos 4,6 s e só voltou ao `latency.discord.media` aos 18,8 s. Na
/// outra, a registrada em `docs/como-funciona.md`, foram 10 s, 12 s e 43 s,
/// com chamadas de API ao `discord.com` no meio segurando o relógio. Com dez
/// segundos a janela fechava no meio das duas sequências e metade da abertura
/// saía pelo IP brasileiro; com trinta, sobrou folga nas duas.
///
/// A medição continua valendo com todo o Discord desviado porque o relógio
/// só ouve esses mesmos três hosts — a CDN passa pelo exterior sem mexer
/// nele, e a voz nem passa.
///
/// Só a primeira conexão do gateway em cada abertura alimenta este relógio —
/// ver `gateway_contado` em `Estado` e o uso em `comecar_aperto`. Sem isso,
/// cada reconexão do gateway pelo proxy gratuito reiniciava a contagem do
/// zero: medidas em produção, quatro reconexões numa abertura só esticaram a
/// janela até bater no teto de 120 s.
const SILENCIO: Duration = Duration::from_secs(30);

/// Teto absoluto da janela. O silêncio é o critério normal; o teto existe para
/// a janela fechar mesmo se algo mantiver as conexões de região vivas.
const TETO: Duration = Duration::from_secs(120);

/// Quantas leituras seguidas sem nenhum `Discord.exe` bastam para aceitar
/// que ele fechou. Uma só não serve: a lista de processos do Windows falha de
/// vez em quando sob carga e volta vazia por um instante, e tratar isso como
/// "fechou" faria a passada seguinte declarar um Discord novo — reabrindo a
/// janela no meio de uma sessão que continua a mesma.
const LEITURAS_ATE_SUMIR: u32 = 3;

/// Um gateway novo depois deste tanto de tempo sem gateway nenhum, com a
/// janela já fechada, é o que parece uma sessão renascendo pelo IP
/// brasileiro — depois de o PC dormir, por exemplo. Reconexão normal reata em
/// segundos e não chega perto disto.
const GATEWAY_MORTO: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fase {
    /// A sessão está nascendo: o IP visto agora é o que vai valer.
    Abertura,
    /// A região já está decidida. Nada mais precisa sair pelo exterior.
    Estabelecida,
}

/// Quem é o Discord no ar: o PID do processo principal e a hora em que ele
/// nasceu. O PID sozinho não basta — o Windows reaproveita PIDs depressa, e
/// um Discord novo pode nascer com o número do antigo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Identidade {
    pub pid: u32,
    pub criado_em: u64,
}

/// O que `observar_discord` viu de novo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mudanca {
    DiscordNovo,
    DiscordFechou,
}

struct Estado {
    fase: Fase,
    /// Apertos de mão de conexões que decidem a região, ainda em curso.
    em_voo: u32,
    armada_em: Instant,
    /// Última vez que uma conexão que decide a região começou ou terminou o
    /// aperto de mão. É daqui que o silêncio conta.
    ultimo_aperto: Option<Instant>,
    discord: Option<Identidade>,
    leituras_vazias: u32,
    /// Conexões com o gateway de pé neste instante, e quando a última caiu.
    gateways: u32,
    gateway_caiu_em: Option<Instant>,
    /// Se a conexão do gateway desta abertura já alimentou o relógio do
    /// silêncio. Uma reconexão dele pelo proxy gratuito continua saindo pelo
    /// exterior e caindo no fechamento — isso é do `socks.rs` — mas só a
    /// primeira soma para o silêncio, senão o proxy derrubando o gateway
    /// repetidas vezes estica a janela até o teto.
    gateway_contado: bool,
}

pub struct Sessao {
    estado: Mutex<Estado>,
    cancelar: broadcast::Sender<()>,
}

/// Um aperto de mão pelo exterior de uma conexão que decide a região, em
/// curso. Enquanto ele existe a janela não vence: um upstream morto segura a
/// chamada por segundos, e o resto da abertura sairia direto. Quando cai — por
/// qualquer caminho, inclusive um `?` no meio — o silêncio recomeça a contar.
///
/// Antes eram duas chamadas soltas em volta do `.await`, e qualquer retorno
/// antecipado entre elas deixava a contagem presa para sempre.
pub struct ApertoDeMao<'a> {
    sessao: &'a Sessao,
}

impl ApertoDeMao<'_> {
    /// Encerra o aperto de mão num instante escolhido — para os testes, que
    /// andam com o relógio na mão. Em produção o guard cai sozinho.
    #[cfg(test)]
    pub fn terminar_em(self, agora: Instant) {
        self.sessao.terminar_aperto(agora);
        std::mem::forget(self);
    }
}

impl Drop for ApertoDeMao<'_> {
    fn drop(&mut self) {
        self.sessao.terminar_aperto(Instant::now());
    }
}

impl Sessao {
    pub fn nova(agora: Instant) -> Self {
        let (cancelar, _) = broadcast::channel(1);
        Self {
            estado: Mutex::new(Estado {
                fase: Fase::Abertura,
                em_voo: 0,
                armada_em: agora,
                ultimo_aperto: None,
                discord: None,
                leituras_vazias: 0,
                gateways: 0,
                gateway_caiu_em: None,
                gateway_contado: false,
            }),
            cancelar,
        }
    }

    /// Um `Mutex` envenenado não pode derrubar o serviço: o pior caso aqui é
    /// uma janela que fecha na hora errada, e isso é melhor do que o Discord
    /// ficar sem proxy nenhum porque uma thread entrou em pânico.
    fn travar(&self) -> std::sync::MutexGuard<'_, Estado> {
        self.estado.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn fase(&self) -> Fase {
        self.travar().fase
    }

    /// Uma conexão começou a ser aberta pelo exterior. Só as que decidem a
    /// região seguram a janela e alimentam o relógio; para as outras — CDN,
    /// voz, anexos — não há guard nenhum, e é isto que deixa a janela fechar
    /// logo depois da abertura mesmo com o Discord em uso.
    pub fn comecar_aperto(&self, host: &str, agora: Instant) -> Option<ApertoDeMao<'_>> {
        if !routing::decide_regiao(host) {
            return None;
        }
        let mut estado = self.travar();
        if routing::e_gateway(host) {
            if estado.gateway_contado {
                return None;
            }
            estado.gateway_contado = true;
        }
        estado.em_voo += 1;
        estado.ultimo_aperto = Some(agora);
        Some(ApertoDeMao { sessao: self })
    }

    /// O aperto de mão terminou — com sucesso ou não. É daqui que o silêncio
    /// começa a contar.
    fn terminar_aperto(&self, agora: Instant) {
        let mut estado = self.travar();
        estado.em_voo = estado.em_voo.saturating_sub(1);
        estado.ultimo_aperto = Some(agora);
    }

    /// Fecha a janela se o silêncio ou o teto já bateram. Devolve `true` na
    /// passada em que fechou — e só nela.
    pub fn avaliar(&self, agora: Instant, ha_exterior: bool) -> bool {
        let mut estado = self.travar();
        if estado.fase == Fase::Estabelecida {
            return false;
        }

        // Duas situações em que a janela não pode nem começar a contar.
        //
        // Sem proxy validado não há para onde desviar: no logon o serviço sobe
        // antes de a piscina encher, e vencer nesse vão faria o Discord abrir
        // sem correção pela sessão inteira.
        //
        // Sem Discord no ar não há sessão nascendo — e a janela precisa estar
        // aberta *antes* de ele voltar. O vigia olha de segundo em segundo, e o
        // Discord fala com o `discord.com` antes disso: medido, a primeira
        // conexão chegou em 2,4 s e o vigia só notou aos 2,6 s. Manter a janela
        // aberta enquanto ele está fechado elimina essa corrida.
        if !ha_exterior || estado.discord.is_none() {
            estado.armada_em = agora;
            estado.ultimo_aperto = None;
            return false;
        }

        // Sem nenhuma conexão de região ainda, o silêncio conta desde que a
        // janela abriu — senão uma janela re-armada num Discord parado ficaria
        // aberta para sempre.
        let referencia = estado.ultimo_aperto.unwrap_or(estado.armada_em);
        let calou =
            estado.em_voo == 0 && agora.saturating_duration_since(referencia) >= SILENCIO;
        let estourou = agora.saturating_duration_since(estado.armada_em) >= TETO;
        if !calou && !estourou {
            return false;
        }

        estado.fase = Fase::Estabelecida;
        drop(estado);

        // Derruba quem ficou preso no exterior. Falha quando não há ninguém
        // escutando, que é justamente o caso em que não havia o que derrubar.
        let _ = self.cancelar.send(());
        true
    }

    /// Compara o Discord principal de agora com o da última passada. Re-arma
    /// quando ele é outro — um Discord novo tem sessão nova — e quando ele
    /// sumiu de vez, para a janela já estar de pé na próxima abertura.
    ///
    /// Filho que nasce ou morre no meio do uso não aparece aqui: só o processo
    /// principal identifica o Discord, e é por isso que um Ctrl+R não re-arma.
    /// Uma leitura vazia também não basta para dizer que ele fechou — ver
    /// `LEITURAS_ATE_SUMIR`.
    pub fn observar_discord(&self, principal: Option<Identidade>, agora: Instant) -> Option<Mudanca> {
        let mut estado = self.travar();
        match principal {
            Some(atual) => {
                estado.leituras_vazias = 0;
                if estado.discord == Some(atual) {
                    return None;
                }
                // Outro processo principal — ou o mesmo PID renascido — é outro
                // Discord, e a região dele ainda vai ser decidida.
                estado.discord = Some(atual);
                Self::abrir(&mut estado, agora);
                Some(Mudanca::DiscordNovo)
            }
            None => {
                // Já não havia Discord: nada mudou.
                estado.discord?;
                estado.leituras_vazias += 1;
                if estado.leituras_vazias < LEITURAS_ATE_SUMIR {
                    return None;
                }
                estado.discord = None;
                estado.leituras_vazias = 0;
                Self::abrir(&mut estado, agora);
                Some(Mudanca::DiscordFechou)
            }
        }
    }

    /// Uma conexão com o gateway começou a ser encaminhada. Devolve há quanto
    /// tempo não havia gateway nenhum quando isso levanta suspeita: a janela
    /// já fechada, o gateway anterior morto há mais de `GATEWAY_MORTO`, e este
    /// é o primeiro a voltar. Uma reconexão normal reata em segundos com
    /// `RESUME` e mantém a região; uma que só vem depois de um vão longo — o PC
    /// dormiu, a internet caiu por minutos — pode ter nascido do zero, e aí a
    /// região foi decidida pelo IP brasileiro.
    ///
    /// Só se registra. O programa não consegue ler a região da sessão em
    /// curso, então reiniciar o Discord por conta disto seria reiniciá-lo por
    /// palpite — e um reinício indevido é pior do que não ter o aviso.
    pub fn gateway_abriu(&self, agora: Instant) -> Option<Duration> {
        let mut estado = self.travar();
        estado.gateways += 1;
        if estado.fase != Fase::Estabelecida || estado.gateways != 1 {
            return None;
        }
        let vao = agora.saturating_duration_since(estado.gateway_caiu_em?);
        (vao >= GATEWAY_MORTO).then_some(vao)
    }

    /// A conexão com o gateway terminou.
    pub fn gateway_fechou(&self, agora: Instant) {
        let mut estado = self.travar();
        estado.gateways = estado.gateways.saturating_sub(1);
        if estado.gateways == 0 {
            estado.gateway_caiu_em = Some(agora);
        }
    }

    /// Assinatura para uma conexão que está saindo pelo exterior. Quando a
    /// janela fechar, ela é avisada e se derruba.
    pub fn assinar_cancelamento(&self) -> broadcast::Receiver<()> {
        self.cancelar.subscribe()
    }

    /// Reabre a janela: a sessão do Discord é outra, e a região dela ainda
    /// vai ser decidida.
    fn abrir(estado: &mut Estado, agora: Instant) {
        estado.fase = Fase::Abertura;
        estado.armada_em = agora;
        estado.ultimo_aperto = None;
        estado.gateway_contado = false;
    }

    #[cfg(test)]
    fn em_voo(&self) -> u32 {
        self.travar().em_voo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COM_EXTERIOR: bool = true;
    const SEM_EXTERIOR: bool = false;

    fn t0() -> Instant {
        Instant::now()
    }

    fn id(pid: u32) -> Option<Identidade> {
        Some(Identidade { pid, criado_em: 1 })
    }

    /// Uma sessão com o Discord no ar, que é a única situação em que a janela
    /// tem o que corrigir.
    fn com_discord(t: Instant) -> Sessao {
        let s = Sessao::nova(t);
        s.observar_discord(id(100), t);
        s
    }

    /// Uma conexão que decide a região: abriu e fechou o aperto de mão.
    fn decisao(s: &Sessao, t: Instant) {
        s.comecar_aperto("discord.com", t)
            .expect("discord.com decide a região")
            .terminar_em(t);
    }

    /// Fecha o Discord de vez: as leituras vazias que o vigia exige.
    fn fechar_discord(s: &Sessao, t: Instant) -> Option<Mudanca> {
        let mut ultima = None;
        for i in 0..LEITURAS_ATE_SUMIR {
            ultima = s.observar_discord(None, t + Duration::from_secs(u64::from(i)));
        }
        ultima
    }

    #[test]
    fn a_janela_nao_vence_com_o_discord_fechado() {
        let t = t0();
        let s = Sessao::nova(t);
        s.observar_discord(None, t);

        // Sem Discord no ar não há sessão nascendo. Deixar a janela vencer aqui
        // faria a próxima abertura pegar a janela já fechada — e a corrida é
        // real: o Discord fala com o discord.com antes de o vigia notar que ele
        // subiu.
        assert!(!s.avaliar(t + Duration::from_secs(3600), COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn a_janela_reabre_quando_o_discord_fecha() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        assert!(s.avaliar(t + SILENCIO, COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Estabelecida);

        // Fechou o Discord: a sessão dele morreu junto, e a próxima vai
        // precisar da correção desde a primeira conexão. Fechar não é um
        // Discord novo — mas reabre a janela assim mesmo.
        assert_eq!(
            fechar_discord(&s, t + Duration::from_secs(60)),
            Some(Mudanca::DiscordFechou)
        );
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn a_janela_espera_a_conexao_em_voo() {
        let t = t0();
        let s = com_discord(t);

        // Um upstream morto segura o aperto de mão por alguns segundos. Se a
        // janela vencesse nesse meio tempo, o resto da abertura do Discord
        // sairia direto e a correção se perderia.
        let aperto = s.comecar_aperto("discord.com", t).unwrap();
        assert!(!s.avaliar(t + SILENCIO + Duration::from_secs(5), COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Abertura);

        let terminou = t + SILENCIO + Duration::from_secs(5);
        aperto.terminar_em(terminou);
        assert!(!s.avaliar(terminou, COM_EXTERIOR), "o silêncio recomeça agora");
        assert!(s.avaliar(terminou + SILENCIO, COM_EXTERIOR));
    }

    #[test]
    fn o_aperto_de_mao_solta_em_qualquer_saida() {
        let t = t0();
        let s = com_discord(t);

        // Um `?` no meio do caminho. Antes, com duas chamadas soltas em volta
        // do await, isto deixava a contagem presa em +1 para sempre — e o
        // critério de silêncio desligado pelo resto da vida do serviço.
        fn abre_e_falha(s: &Sessao, t: Instant) -> Result<(), ()> {
            let _aperto = s
                .comecar_aperto("discord.com", t)
                .expect("discord.com decide a região");
            assert_eq!(s.em_voo(), 1, "o guard existe enquanto o aperto corre");
            Err(())?;
            Ok(())
        }
        assert!(abre_e_falha(&s, t).is_err());
        assert_eq!(s.em_voo(), 0, "o guard soltou ao cair");

        // O guard caiu com o relógio de verdade, que aqui é ~t.
        assert!(s.avaliar(t + SILENCIO + Duration::from_secs(1), COM_EXTERIOR));
    }

    #[test]
    fn a_janela_espera_a_piscina_encher() {
        let t = t0();
        let s = com_discord(t);

        // No logon o serviço sobe antes de ter proxy nenhum validado. Deixar a
        // janela vencer nesse vão faria o Discord abrir sem correção — e a
        // correção só vale na abertura, então ela estaria perdida pela sessão
        // inteira.
        assert!(!s.avaliar(t + Duration::from_secs(600), SEM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Abertura);

        // Encheu: só agora a contagem começa a valer.
        let encheu = t + Duration::from_secs(600);
        assert!(!s.avaliar(encheu + Duration::from_secs(9), COM_EXTERIOR));
        assert!(s.avaliar(encheu + SILENCIO, COM_EXTERIOR));
    }

    #[test]
    fn comeca_em_abertura() {
        assert_eq!(Sessao::nova(t0()).fase(), Fase::Abertura);
    }

    #[test]
    fn silencio_fecha_a_janela() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);

        assert!(s.avaliar(t + SILENCIO, COM_EXTERIOR), "o silêncio completo fecha");
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn trafego_continuo_nao_fecha_antes_do_silencio() {
        let t = t0();
        let s = com_discord(t);

        decisao(&s, t);
        decisao(&s, t + Duration::from_secs(5));

        assert!(!s.avaliar(t + Duration::from_secs(9), COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Abertura, "ainda faltam 26s de silêncio");
        assert!(!s.avaliar(t + SILENCIO, COM_EXTERIOR), "a segunda conexão empurrou o relógio");
        assert!(s.avaliar(t + Duration::from_secs(5) + SILENCIO, COM_EXTERIOR));
    }

    #[test]
    fn so_quem_decide_regiao_alimenta_o_relogio() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);

        // A CDN, a voz e a página de avisos saem pelo exterior na abertura,
        // mas não podem segurar a janela: um Discord em uso abre conexão
        // nova destas o tempo todo, e o silêncio nunca completaria — a janela
        // passaria a fechar sempre pelo teto, dois minutos depois, no meio do
        // uso. Este teste é o que denuncia essa regressão.
        let tarde = t + Duration::from_secs(25);
        for h in [
            "cdn.discordapp.com",
            "c-gru17-851904d3.discord.media",
            "status.discord.com",
        ] {
            assert!(
                s.comecar_aperto(h, tarde).is_none(),
                "{h} não segura a janela"
            );
        }
        assert_eq!(s.em_voo(), 0);
        assert!(
            s.avaliar(t + SILENCIO, COM_EXTERIOR),
            "o relógio ficou parado em t: a janela fecha aos 30s"
        );

        // E o contraponto: uma conexão que decide a região no mesmo instante
        // teria empurrado o relógio.
        let s = com_discord(t);
        decisao(&s, t);
        decisao(&s, tarde);
        assert!(!s.avaliar(t + SILENCIO, COM_EXTERIOR));
        assert!(s.avaliar(tarde + SILENCIO, COM_EXTERIOR));
    }

    #[test]
    fn o_teto_fecha_mesmo_com_trafego_continuo() {
        let t = t0();
        let s = com_discord(t);

        // Uma conexão de região a cada 5s, para sempre: o silêncio nunca
        // completa sozinho e só o teto encerra a janela.
        let mut passo = Duration::ZERO;
        while passo < TETO {
            decisao(&s, t + passo);
            assert!(!s.avaliar(t + passo, COM_EXTERIOR), "não pode fechar antes do teto");
            passo += Duration::from_secs(5);
        }

        assert!(s.avaliar(t + TETO, COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn a_janela_so_fecha_uma_vez() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);

        assert!(s.avaliar(t + SILENCIO, COM_EXTERIOR), "primeira passada fecha");
        assert!(
            !s.avaliar(t + SILENCIO + Duration::from_secs(1), COM_EXTERIOR),
            "já estava fechada; não fecha de novo"
        );
    }

    #[test]
    fn fechar_avisa_quem_esta_no_exterior() {
        let t = t0();
        let s = com_discord(t);
        let mut assinatura = s.assinar_cancelamento();
        decisao(&s, t);

        assert!(
            assinatura.try_recv().is_err(),
            "com a janela aberta ninguém é derrubado"
        );

        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        assert!(assinatura.try_recv().is_ok(), "fechou, então derruba");
        assert!(
            assinatura.try_recv().is_err(),
            "um aviso só; a conexão não é derrubada duas vezes"
        );
    }

    #[test]
    fn discord_reiniciado_rearma() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);
        assert_eq!(s.fase(), Fase::Estabelecida);

        // Outro processo principal: é outro Discord, com outra sessão.
        assert_eq!(
            s.observar_discord(id(300), t + Duration::from_secs(60)),
            Some(Mudanca::DiscordNovo)
        );
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn pid_reaproveitado_ainda_e_discord_novo() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        // O Windows devolveu o mesmo número ao Discord novo. A hora de criação
        // é o que denuncia que não é o mesmo processo.
        let renascido = Some(Identidade { pid: 100, criado_em: 2 });
        assert_eq!(
            s.observar_discord(renascido, t + Duration::from_secs(60)),
            Some(Mudanca::DiscordNovo)
        );
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn o_mesmo_discord_nao_rearma() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        // Renderizador morreu, nasceu outro, Ctrl+R: o principal continua lá,
        // a sessão é a mesma e não há nada a corrigir.
        assert_eq!(s.observar_discord(id(100), t + Duration::from_secs(20)), None);
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn discord_abrindo_do_zero_rearma() {
        let t = t0();
        let s = Sessao::nova(t);
        s.observar_discord(None, t);

        assert_eq!(
            s.observar_discord(id(500), t + Duration::from_secs(30)),
            Some(Mudanca::DiscordNovo),
            "Discord aparecendo do nada é um Discord novo"
        );
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn leitura_vazia_transitoria_nao_rearma() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        // A lista de processos falhou por um instante e voltou vazia. Antes,
        // isto zerava o conjunto conhecido e a passada seguinte — com o mesmo
        // Discord de sempre — era lida como "Discord reiniciou".
        let depois = t + Duration::from_secs(60);
        assert_eq!(s.observar_discord(None, depois), None);
        assert_eq!(s.fase(), Fase::Estabelecida);
        assert_eq!(s.observar_discord(None, depois + Duration::from_secs(1)), None);
        assert_eq!(s.fase(), Fase::Estabelecida);

        assert_eq!(
            s.observar_discord(id(100), depois + Duration::from_secs(2)),
            None,
            "o mesmo Discord voltou a aparecer: nada mudou"
        );
        assert_eq!(s.fase(), Fase::Estabelecida);

        // Só a sequência inteira de leituras vazias fecha.
        assert_eq!(
            fechar_discord(&s, depois + Duration::from_secs(10)),
            Some(Mudanca::DiscordFechou)
        );
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn discord_novo_no_meio_das_leituras_vazias_e_percebido_na_hora() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        // Um reinício rápido: uma leitura vazia e o Discord novo já está lá.
        // Não precisa esperar a contagem de leituras vazias completar.
        let depois = t + Duration::from_secs(60);
        assert_eq!(s.observar_discord(None, depois), None);
        assert_eq!(
            s.observar_discord(id(300), depois + Duration::from_secs(1)),
            Some(Mudanca::DiscordNovo)
        );
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn rearmar_reabre_a_contagem_do_silencio() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        let depois = t + Duration::from_secs(60);
        s.observar_discord(id(200), depois);
        assert_eq!(s.fase(), Fase::Abertura);

        // A janela nova é inteira: o silêncio conta a partir do re-armar, não
        // do tráfego da sessão antiga.
        assert!(!s.avaliar(depois + Duration::from_secs(9), COM_EXTERIOR));
        assert!(s.avaliar(depois + SILENCIO, COM_EXTERIOR));
    }

    #[test]
    fn reconexao_do_gateway_nao_empurra_o_relogio() {
        let t = t0();
        let s = com_discord(t);

        // O primeiro gateway da abertura conta normalmente.
        s.comecar_aperto("gateway.discord.gg", t)
            .expect("o primeiro gateway decide a região")
            .terminar_em(t);

        // O proxy gratuito derruba o gateway e o Discord reconecta pelo
        // exterior de novo. Cada reconexão dessas era uma conexão nova que
        // decidia região e reiniciava o silêncio — quatro reconexões numa
        // abertura só, medidas em produção, esticavam a janela até o teto.
        assert!(
            s.comecar_aperto("gateway.discord.gg", t + Duration::from_secs(25))
                .is_none(),
            "a reconexão do gateway não segura a janela nem empurra o relógio"
        );

        assert!(s.avaliar(t + Duration::from_secs(30), COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn gateway_novo_depois_de_um_vao_longo_e_suspeito() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);
        assert_eq!(s.fase(), Fase::Estabelecida);

        // O primeiro gateway depois da janela: não há vão anterior para medir.
        let g1 = t + Duration::from_secs(31);
        assert_eq!(s.gateway_abriu(g1), None);

        // Caiu e reatou em segundos — RESUME, mesma sessão, mesma região.
        s.gateway_fechou(g1 + Duration::from_secs(600));
        let g2 = g1 + Duration::from_secs(603);
        assert_eq!(s.gateway_abriu(g2), None);

        // Caiu e só voltou um minuto depois: o PC dormiu, a sessão pode ter
        // renascido pelo IP brasileiro. É o que se registra.
        s.gateway_fechou(g2 + Duration::from_secs(600));
        let g3 = g2 + Duration::from_secs(600) + GATEWAY_MORTO;
        assert_eq!(s.gateway_abriu(g3), Some(GATEWAY_MORTO));
    }

    #[test]
    fn gateway_novo_na_abertura_nao_e_suspeito() {
        let t = t0();
        let s = com_discord(t);

        // Com a janela aberta, o gateway novo é a própria abertura sendo
        // corrigida — por mais longo que tenha sido o vão.
        s.gateway_fechou(t);
        assert_eq!(s.gateway_abriu(t + GATEWAY_MORTO * 10), None);
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn segundo_gateway_com_o_primeiro_de_pe_nao_e_suspeito() {
        let t = t0();
        let s = com_discord(t);
        decisao(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        let g1 = t + Duration::from_secs(31);
        s.gateway_abriu(g1);
        s.gateway_fechou(g1 + Duration::from_secs(10));
        let g2 = g1 + Duration::from_secs(10) + GATEWAY_MORTO;
        assert!(s.gateway_abriu(g2).is_some(), "o primeiro depois do vão avisa");

        // Enquanto esse está de pé, um segundo não é um vão: é só outra conexão.
        assert_eq!(s.gateway_abriu(g2 + Duration::from_secs(1)), None);
    }
}

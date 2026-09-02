//! Em que momento da vida da sessão o Discord está.
//!
//! O IP estrangeiro só é lido uma vez, quando a sessão nasce; daí em diante a
//! região já está decidida e gravada nela. Então o desvio pelo exterior só faz
//! sentido durante a abertura — depois dela, cada conexão que continuasse
//! saindo por fora seria latência pura, sem comprar correção nenhuma.
//!
//! Este módulo é quem sabe a diferença. Ele abre uma janela quando o Discord
//! aparece, fecha quando a rajada de abertura passa, e derruba o que ficou
//! preso no exterior para o Discord reconectar direto — o mesmo efeito de
//! desligar a VPN depois de abrir o programa, que é a correção manual que este
//! projeto automatiza.

use std::{
    collections::HashSet,
    sync::Mutex,
    time::{Duration, Instant},
};
use tokio::sync::broadcast;

/// Silêncio que declara a rajada de abertura encerrada.
///
/// Trinta segundos vêm de medir a abertura de verdade nesta máquina: o Discord
/// falou com o `discord.com` em 2,4 s, com o gateway em 4,6 s e só voltou ao
/// `latency.discord.media` aos 18,8 s. Com dez segundos a janela fechava no
/// meio dessa sequência e metade da abertura saía pelo IP brasileiro.
const SILENCIO: Duration = Duration::from_secs(30);

/// Teto absoluto da janela. O silêncio é o critério normal; o teto existe para
/// a janela fechar mesmo se algo mantiver o tráfego de controle vivo.
const TETO: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fase {
    /// A sessão está nascendo: o IP visto agora é o que vai valer.
    Abertura,
    /// A região já está decidida. Nada mais precisa sair pelo exterior.
    Estabelecida,
}

struct Estado {
    fase: Fase,
    em_voo: u32,
    armada_em: Instant,
    ultima_controle: Option<Instant>,
    pids_discord: HashSet<u32>,
}

pub struct Sessao {
    estado: Mutex<Estado>,
    cancelar: broadcast::Sender<()>,
}

impl Sessao {
    pub fn nova(agora: Instant) -> Self {
        let (cancelar, _) = broadcast::channel(1);
        Self {
            estado: Mutex::new(Estado {
                fase: Fase::Abertura,
                em_voo: 0,
                armada_em: agora,
                ultima_controle: None,
                pids_discord: HashSet::new(),
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

    /// Uma conexão de controle começou a ser aberta pelo exterior.
    pub fn entrar_controle(&self, agora: Instant) {
        let mut estado = self.travar();
        estado.em_voo += 1;
        estado.ultima_controle = Some(agora);
    }

    /// A conexão de controle terminou de ser estabelecida — com sucesso ou
    /// não. É daqui que o silêncio começa a contar.
    pub fn sair_controle(&self, agora: Instant) {
        let mut estado = self.travar();
        estado.em_voo = estado.em_voo.saturating_sub(1);
        estado.ultima_controle = Some(agora);
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
        // antes da piscina encher, e vencer nesse vão faria o Discord abrir sem
        // correção pela sessão inteira.
        //
        // Sem Discord no ar não há sessão nascendo — e a janela precisa estar
        // aberta *antes* de ele voltar. O vigia olha de segundo em segundo, e o
        // Discord fala com o `discord.com` antes disso: medido, a primeira
        // conexão chegou em 2,4 s e o vigia só notou aos 2,6 s. Manter a janela
        // aberta enquanto ele está fechado elimina essa corrida.
        if !ha_exterior || estado.pids_discord.is_empty() {
            estado.armada_em = agora;
            estado.ultima_controle = None;
            return false;
        }

        // Sem nenhuma conexão de controle ainda, o silêncio conta desde que a
        // janela abriu — senão uma janela re-armada num Discord parado ficaria
        // aberta para sempre.
        let referencia = estado.ultima_controle.unwrap_or(estado.armada_em);
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

    /// Compara os processos do Discord com os da última passada. Re-arma
    /// quando nenhum dos anteriores sobrou, que é o que distingue um reinício
    /// de verdade de um renderizador que nasceu ou morreu no meio do uso.
    pub fn observar_discord(&self, pids: &[u32], agora: Instant) -> bool {
        let atuais: HashSet<u32> = pids.iter().copied().collect();
        let mut estado = self.travar();

        // Um Discord novo tem sessão nova. Discord que fechou também reabre a
        // janela: a próxima abertura vai precisar dela desde a primeira
        // conexão, e é `avaliar` que a segura aberta enquanto ele não volta.
        let fechou = atuais.is_empty() && !estado.pids_discord.is_empty();
        let trocou = !atuais.is_empty() && atuais.is_disjoint(&estado.pids_discord);
        estado.pids_discord = atuais;
        if trocou || fechou {
            Self::abrir(&mut estado, agora);
        }
        trocou
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
        estado.ultima_controle = None;
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

    /// Uma sessão com o Discord no ar, que é a única situação em que a janela
    /// tem o que corrigir.
    fn com_discord(t: Instant) -> Sessao {
        let s = Sessao::nova(t);
        s.observar_discord(&[100], t);
        s
    }

    /// Uma conexão de controle que abriu e fechou o aperto de mão.
    fn controle(s: &Sessao, t: Instant) {
        s.entrar_controle(t);
        s.sair_controle(t);
    }

    #[test]
    fn a_janela_nao_vence_com_o_discord_fechado() {
        let t = t0();
        let s = Sessao::nova(t);
        s.observar_discord(&[], t);

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
        controle(&s, t);
        assert!(s.avaliar(t + SILENCIO, COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Estabelecida);

        // Fechou o Discord: a sessão dele morreu junto, e a próxima vai
        // precisar da correção desde a primeira conexão.
        s.observar_discord(&[], t + Duration::from_secs(60));
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn a_janela_espera_a_conexao_em_voo() {
        let t = t0();
        let s = com_discord(t);

        // Um upstream morto segura o aperto de mão por até 30s. Se a janela
        // vencesse nesse meio tempo, o resto da abertura do Discord sairia
        // direto e a correção se perderia.
        s.entrar_controle(t);
        assert!(!s.avaliar(t + SILENCIO + Duration::from_secs(5), COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Abertura);

        let terminou = t + SILENCIO + Duration::from_secs(5);
        s.sair_controle(terminou);
        assert!(!s.avaliar(terminou, COM_EXTERIOR), "o silêncio recomeça agora");
        assert!(s.avaliar(terminou + SILENCIO, COM_EXTERIOR));
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
        controle(&s, t);

        assert!(s.avaliar(t + SILENCIO, COM_EXTERIOR), "o silêncio completo fecha");
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn trafego_continuo_nao_fecha_antes_do_silencio() {
        let t = t0();
        let s = Sessao::nova(t);

        controle(&s, t);
        controle(&s, t + Duration::from_secs(5));

        assert!(!s.avaliar(t + Duration::from_secs(9), COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Abertura, "ainda faltam 6s de silêncio");
    }

    #[test]
    fn o_teto_fecha_mesmo_com_trafego_continuo() {
        let t = t0();
        let s = com_discord(t);

        // Uma conexão de controle a cada 5s, para sempre: o silêncio nunca
        // completa sozinho e só o teto encerra a janela.
        let mut passo = Duration::ZERO;
        while passo < TETO {
            controle(&s, t + passo);
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
        controle(&s, t);

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
        controle(&s, t);

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
        let s = Sessao::nova(t);
        s.observar_discord(&[100, 101], t);
        controle(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);
        assert_eq!(s.fase(), Fase::Estabelecida);

        // Nenhum PID anterior sobrou: é outro Discord, com outra sessão.
        assert!(s.observar_discord(&[300, 301], t + Duration::from_secs(60)));
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn renderizador_novo_nao_rearma() {
        let t = t0();
        let s = Sessao::nova(t);
        s.observar_discord(&[100, 101], t);
        controle(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        // O 101 morreu e nasceu o 102, mas o processo principal continua lá:
        // a sessão é a mesma e não há nada a corrigir.
        assert!(!s.observar_discord(&[100, 102], t + Duration::from_secs(20)));
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn discord_abrindo_do_zero_rearma() {
        let t = t0();
        let s = Sessao::nova(t);
        s.observar_discord(&[], t);

        assert!(
            s.observar_discord(&[500], t + Duration::from_secs(30)),
            "Discord aparecendo do nada é um Discord novo"
        );
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn discord_fechado_nao_conta_como_discord_novo() {
        let t = t0();
        let s = com_discord(t);
        controle(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        // Fechar não é um Discord novo, então não é reinício — mas a janela
        // reabre assim mesmo, para estar pronta quando ele voltar.
        assert!(!s.observar_discord(&[], t + Duration::from_secs(30)));
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn rearmar_reabre_a_contagem_do_silencio() {
        let t = t0();
        let s = com_discord(t);
        controle(&s, t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        let depois = t + Duration::from_secs(60);
        s.observar_discord(&[200], depois);
        assert_eq!(s.fase(), Fase::Abertura);

        // A janela nova é inteira: o silêncio conta a partir do re-armar, não
        // do tráfego da sessão antiga.
        assert!(!s.avaliar(depois + Duration::from_secs(9), COM_EXTERIOR));
        assert!(s.avaliar(depois + SILENCIO, COM_EXTERIOR));
    }
}

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

/// Silêncio que declara a rajada de abertura encerrada. Curto o bastante para
/// a correção não custar latência à toa, longo o bastante para não cortar um
/// login no meio numa máquina lenta.
const SILENCIO: Duration = Duration::from_secs(10);

/// Teto absoluto da janela. O silêncio é o critério normal; o teto existe para
/// a janela fechar mesmo se algo mantiver o tráfego de controle vivo.
const TETO: Duration = Duration::from_secs(90);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fase {
    /// A sessão está nascendo: o IP visto agora é o que vai valer.
    Abertura,
    /// A região já está decidida. Nada mais precisa sair pelo exterior.
    Estabelecida,
}

struct Estado {
    fase: Fase,
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

    /// Anota que passou uma conexão dos hosts que decidem a região. É o que
    /// alimenta o silêncio.
    pub fn registrar_controle(&self, agora: Instant) {
        self.travar().ultima_controle = Some(agora);
    }

    /// Fecha a janela se o silêncio ou o teto já bateram. Devolve `true` na
    /// passada em que fechou — e só nela.
    pub fn avaliar(&self, agora: Instant, ha_exterior: bool) -> bool {
        let mut estado = self.travar();
        if estado.fase == Fase::Estabelecida {
            return false;
        }

        // No logon o serviço sobe antes de ter validado o primeiro proxy. Se a
        // janela vencesse nesse vão, o Discord abriria sem correção — e como a
        // correção só existe na abertura, ela estaria perdida pela sessão
        // inteira. Enquanto não há para onde desviar, a janela não conta.
        if !ha_exterior {
            estado.armada_em = agora;
            estado.ultima_controle = None;
            return false;
        }

        // Sem nenhuma conexão de controle ainda, o silêncio conta desde que a
        // janela abriu — senão uma janela re-armada num Discord parado ficaria
        // aberta para sempre.
        let referencia = estado.ultima_controle.unwrap_or(estado.armada_em);
        let calou = agora.saturating_duration_since(referencia) >= SILENCIO;
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

        // Discord fechado não tem sessão para corrigir: gastar a janela agora
        // só a deixaria vencida quando ele voltasse.
        let trocou = !atuais.is_empty() && atuais.is_disjoint(&estado.pids_discord);
        estado.pids_discord = atuais;
        if trocou {
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

    #[test]
    fn a_janela_espera_a_piscina_encher() {
        let t = t0();
        let s = Sessao::nova(t);

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
        let s = Sessao::nova(t);
        s.registrar_controle(t);

        assert!(s.avaliar(t + SILENCIO, COM_EXTERIOR), "o silêncio completo fecha");
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn trafego_continuo_nao_fecha_antes_do_silencio() {
        let t = t0();
        let s = Sessao::nova(t);

        s.registrar_controle(t);
        s.registrar_controle(t + Duration::from_secs(5));

        assert!(!s.avaliar(t + Duration::from_secs(9), COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Abertura, "ainda faltam 6s de silêncio");
    }

    #[test]
    fn o_teto_fecha_mesmo_com_trafego_continuo() {
        let t = t0();
        let s = Sessao::nova(t);

        // Uma conexão de controle a cada 5s, para sempre: o silêncio nunca
        // completa sozinho e só o teto encerra a janela.
        let mut passo = Duration::ZERO;
        while passo < TETO {
            s.registrar_controle(t + passo);
            assert!(!s.avaliar(t + passo, COM_EXTERIOR), "não pode fechar antes do teto");
            passo += Duration::from_secs(5);
        }

        assert!(s.avaliar(t + TETO, COM_EXTERIOR));
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn a_janela_so_fecha_uma_vez() {
        let t = t0();
        let s = Sessao::nova(t);
        s.registrar_controle(t);

        assert!(s.avaliar(t + SILENCIO, COM_EXTERIOR), "primeira passada fecha");
        assert!(
            !s.avaliar(t + SILENCIO + Duration::from_secs(1), COM_EXTERIOR),
            "já estava fechada; não fecha de novo"
        );
    }

    #[test]
    fn fechar_avisa_quem_esta_no_exterior() {
        let t = t0();
        let s = Sessao::nova(t);
        let mut assinatura = s.assinar_cancelamento();
        s.registrar_controle(t);

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
        s.registrar_controle(t);
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
        s.registrar_controle(t);
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
        s.registrar_controle(t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);
        assert_eq!(s.fase(), Fase::Estabelecida);

        assert!(s.observar_discord(&[500], t + Duration::from_secs(30)));
        assert_eq!(s.fase(), Fase::Abertura);
    }

    #[test]
    fn discord_fechado_nao_rearma() {
        let t = t0();
        let s = Sessao::nova(t);
        s.observar_discord(&[100], t);
        s.registrar_controle(t);
        s.avaliar(t + SILENCIO, COM_EXTERIOR);

        // Sem Discord no ar não há sessão para corrigir; re-armar agora só
        // gastaria a janela antes de ele voltar.
        assert!(!s.observar_discord(&[], t + Duration::from_secs(30)));
        assert_eq!(s.fase(), Fase::Estabelecida);
    }

    #[test]
    fn rearmar_reabre_a_contagem_do_silencio() {
        let t = t0();
        let s = Sessao::nova(t);
        s.observar_discord(&[100], t);
        s.registrar_controle(t);
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

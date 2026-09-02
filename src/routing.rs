//! Decide, por host, se a conexão sai pelo exterior ou direto.
//!
//! O modelo é o de uma VPN que se liga para abrir o Discord e se desliga
//! assim que ele entrou. Enquanto a sessão está nascendo, **todo** o domínio
//! do Discord sai por um IP estrangeiro — API, gateway, CDN, anexos e o TCP
//! dos servidores de voz. Com a sessão aberta, a região já está gravada nela
//! e tudo volta a sair direto.
//!
//! Duas listas, com papéis diferentes, que não podem ser confundidas:
//!
//! - `DISCORD` diz o que é **desviado**. É sempre uma lista de domínios do
//!   Discord, nunca "qualquer host": o SOCKS local aceita conexão de qualquer
//!   programa da máquina, e devolver `Exterior` sem olhar o host o
//!   transformaria num relay estrangeiro de uso geral durante a janela.
//! - `DECIDE_REGIAO` diz o que **alimenta o relógio** que fecha a janela. São
//!   só os hosts cujo IP de origem decide a região. Se a CDN também contasse,
//!   um Discord em uso — trocando de canal, carregando imagem — nunca deixaria
//!   o silêncio completar, e a janela só fecharia pelo teto, no meio do uso.

use crate::sessao::Fase;

/// Todo o domínio do Discord. É o que sai pelo exterior enquanto a sessão
/// está nascendo. `discordapp.net` não está aqui porque o PAC nunca o entrega
/// ao proxy local — ele sai direto antes de chegar a este código.
const DISCORD: &[&str] = &["discord.com", "discordapp.com", "discord.gg", "discord.media"];

/// Hosts cujo IP de origem decide a região da sessão. Só eles alimentam o
/// relógio do silêncio que fecha a janela.
const DECIDE_REGIAO: &[&str] = &["discord.com", "gateway.discord.gg", "latency.discord.media"];

/// A página pública de avisos. Casa com `discord.com` e sai pelo exterior
/// junto com o resto durante a abertura, mas não decide região nenhuma — então
/// não segura a janela aberta.
const AVISOS: &str = "status.discord.com";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rota {
    Exterior,
    Direta,
}

fn casa(host: &str, dominio: &str) -> bool {
    host == dominio || host.ends_with(&format!(".{dominio}"))
}

fn normalizar(host: &str) -> String {
    host.trim_end_matches('.').to_ascii_lowercase()
}

/// Só o que um nome DNS pode ter: letras, dígitos, ponto, hífen e o
/// sublinhado de alguns registros. Um nome fora disto nunca casa com um
/// sufixo do Discord — `evil.com\0.discord.com` termina em `.discord.com`,
/// mas um upstream escrito em C resolveria só o `evil.com`.
pub fn nome_bem_formado(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

pub fn decidir(host: &str, fase: Fase) -> Rota {
    // Com a sessão já aberta, a região está decidida e gravada nela. Continuar
    // saindo pelo exterior a partir daqui não compra correção nenhuma — só
    // paga latência, e no cano por onde passam as mensagens.
    if fase == Fase::Estabelecida {
        return Rota::Direta;
    }

    let host = normalizar(host);
    if !nome_bem_formado(&host) {
        return Rota::Direta;
    }
    if DISCORD.iter().any(|d| casa(&host, d)) {
        Rota::Exterior
    } else {
        Rota::Direta
    }
}

/// O gateway aparece com sabor regional no tráfego de verdade —
/// `gateway.discord.gg`, `gateway-us-east1-b.discord.gg`. Todos são a mesma
/// conexão principal, e é por ela que a sessão nasce. O
/// `remote-auth-gateway`, do login por QR code, não é ela.
pub fn e_gateway(host: &str) -> bool {
    let host = normalizar(host);
    casa(&host, "discord.gg")
        && host
            .split('.')
            .next()
            .is_some_and(|rotulo| rotulo.starts_with("gateway"))
}

/// Se o IP de origem desta conexão decide a região da sessão.
pub fn decide_regiao(host: &str) -> bool {
    let host = normalizar(host);
    if casa(&host, AVISOS) {
        return false;
    }
    DECIDE_REGIAO.iter().any(|d| casa(&host, d)) || e_gateway(&host)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISCORD_INTEIRO: &[&str] = &[
        "discord.com",
        "gateway.discord.gg",
        "gateway-us-east1-b.discord.gg",
        "latency.discord.media",
        "cdn.discordapp.com",
        "c-gru17-851904d3.discord.media",
        "status.discord.com",
        "discord.gg",
        "DISCORD.COM.",
    ];

    #[test]
    fn na_abertura_todo_o_discord_sai_por_fora() {
        // É a VPN ligada: API, gateway, CDN, voz por TCP, até a página de
        // avisos. Nada do Discord fica de fora enquanto a sessão nasce.
        for h in DISCORD_INTEIRO {
            assert_eq!(decidir(h, Fase::Abertura), Rota::Exterior, "{h}");
        }
    }

    #[test]
    fn resto_da_internet_vai_direto() {
        // A guarda do desenho: o SOCKS local aceita conexão de qualquer
        // programa da máquina. Se isto ficar vermelho, o proxy virou relay
        // estrangeiro de uso geral durante a janela.
        for h in [
            "google.com",
            "api.spotify.com",
            "discord.com.evil.net",
            // O PAC nunca entrega este domínio ao proxy; se um dia entregar,
            // ele continua indo direto até alguém decidir o contrário aqui.
            "media.discordapp.net",
        ] {
            assert_eq!(decidir(h, Fase::Abertura), Rota::Direta, "{h}");
        }
    }

    #[test]
    fn nao_confunde_sufixo() {
        for h in ["naodiscord.com", "meudiscord.gg", "xdiscord.media"] {
            assert_eq!(decidir(h, Fase::Abertura), Rota::Direta, "{h}");
        }
    }

    #[test]
    fn nome_mal_formado_nunca_sai_por_fora() {
        // Termina em `.discord.com` só na aparência: um upstream em C pararia
        // no byte nulo e resolveria `evil.com`. O SOCKS local recusa isto
        // antes, mas a decisão de rota também não pode cair nessa.
        for h in [
            "evil.com\0.discord.com",
            "evil.com\n.discord.com",
            "a b.discord.com",
            "",
        ] {
            assert_eq!(decidir(h, Fase::Abertura), Rota::Direta, "{h:?}");
        }
    }

    #[test]
    fn com_a_sessao_aberta_tudo_vai_direto() {
        // A VPN desligada: a região já está gravada na sessão, e cada conexão
        // que continuasse saindo por fora seria latência pura.
        for h in DISCORD_INTEIRO {
            assert_eq!(decidir(h, Fase::Estabelecida), Rota::Direta, "{h}");
        }
    }

    #[test]
    fn so_quem_decide_regiao_alimenta_o_relogio() {
        for h in [
            "discord.com",
            "Discord.com",
            "gateway.discord.gg",
            "gateway-us-east1-b.discord.gg",
            "latency.discord.media",
        ] {
            assert!(decide_regiao(h), "{h} decide a região");
        }

        // Tudo isto sai pelo exterior na abertura, mas não pode segurar a
        // janela: um Discord em uso abre conexão nova destas o tempo todo.
        for h in [
            "cdn.discordapp.com",
            "c-gru17-851904d3.discord.media",
            "status.discord.com",
            "discord.gg",
            "discordapp.com",
            "remote-auth-gateway.discord.gg",
            "google.com",
        ] {
            assert!(!decide_regiao(h), "{h} não decide a região");
        }
    }

    #[test]
    fn o_que_decide_regiao_tambem_e_desviado() {
        // Não pode existir host que segure a janela aberta sem sair por fora.
        for h in ["discord.com", "gateway-us-east1-b.discord.gg", "latency.discord.media"] {
            assert!(decide_regiao(h));
            assert_eq!(decidir(h, Fase::Abertura), Rota::Exterior, "{h}");
        }
    }

    #[test]
    fn reconhece_o_gateway_em_todos_os_sabores() {
        assert!(e_gateway("gateway.discord.gg"));
        assert!(e_gateway("gateway-us-east1-b.discord.gg"));
        assert!(!e_gateway("remote-auth-gateway.discord.gg"));
        assert!(!e_gateway("discord.gg"));
        assert!(!e_gateway("gateway.discord.com"));
    }
}

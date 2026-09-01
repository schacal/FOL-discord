//! Decide, por host, se a conexão sai pelo exterior ou direto.
//!
//! Só os endpoints que determinam a geolocalização da sessão precisam sair
//! por fora. Tudo o mais — CDN, voz, o resto da internet — vai direto, que é
//! o que mantém o ping normal e a transmissão de tela funcionando.

/// Hosts cujo IP de origem define a região que o Discord atribui à sessão.
const CONTROLE: &[&str] = &[
    "discord.com",
    "discordapp.com",
    "gateway.discord.gg",
    "discord.gg",
    "latency.discord.media",
];

/// Hosts que nunca devem sair por fora, mesmo que casem com a lista acima.
/// A CDN é volume puro e a voz precisa de rota curta.
const NUNCA: &[&str] = &[
    "cdn.discordapp.com",
    "media.discordapp.net",
    "images-ext-1.discordapp.net",
    "images-ext-2.discordapp.net",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rota {
    Exterior,
    Direta,
}

/// Modo de operação escolhido na linha de comando.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Modo {
    /// Só os endpoints de controle saem por fora. Padrão.
    Controle,
    /// Todo domínio do Discord sai por fora. Rede de segurança caso o
    /// conjunto mínimo não baste em alguma máquina.
    TudoDiscord,
}

fn casa(host: &str, dominio: &str) -> bool {
    host == dominio || host.ends_with(&format!(".{dominio}"))
}

pub fn decidir(host: &str, modo: Modo) -> Rota {
    let host = host.trim_end_matches('.').to_ascii_lowercase();

    if NUNCA.iter().any(|d| casa(&host, d)) {
        return Rota::Direta;
    }

    let alvo: &[&str] = match modo {
        Modo::Controle => CONTROLE,
        Modo::TudoDiscord => &["discord.com", "discordapp.com", "discord.gg", "discord.media"],
    };

    if alvo.iter().any(|d| casa(&host, d)) {
        Rota::Exterior
    } else {
        Rota::Direta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn controle_sai_por_fora() {
        for h in ["discord.com", "gateway.discord.gg", "latency.discord.media"] {
            assert_eq!(decidir(h, Modo::Controle), Rota::Exterior, "{h}");
        }
    }

    #[test]
    fn cdn_e_voz_vao_direto() {
        for h in [
            "cdn.discordapp.com",
            "media.discordapp.net",
            "c-gru17-851904d3.discord.media",
        ] {
            assert_eq!(decidir(h, Modo::Controle), Rota::Direta, "{h}");
        }
    }

    #[test]
    fn resto_da_internet_vai_direto() {
        for h in ["google.com", "api.spotify.com", "discord.com.evil.net"] {
            assert_eq!(decidir(h, Modo::Controle), Rota::Direta, "{h}");
        }
    }

    #[test]
    fn modo_tudo_discord_pega_a_voz_mas_nunca_a_cdn() {
        assert_eq!(
            decidir("c-gru17-851904d3.discord.media", Modo::TudoDiscord),
            Rota::Exterior
        );
        assert_eq!(decidir("cdn.discordapp.com", Modo::TudoDiscord), Rota::Direta);
    }

    #[test]
    fn nao_confunde_sufixo() {
        assert_eq!(decidir("naodiscord.com", Modo::Controle), Rota::Direta);
    }
}

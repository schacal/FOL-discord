//! Servidor do arquivo PAC.
//!
//! O Windows lê este arquivo em toda abertura do Discord. É por isso que a
//! correção sobrevive a reinícios e a atualizações do Discord sem tocar em
//! atalho nenhum. Só o tráfego do Discord é entregue ao proxy local; o resto
//! da internet nem passa por aqui.

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub fn texto(porta_socks: u16) -> String {
    format!(
        r#"function FindProxyForURL(url, host) {{
  if (dnsDomainIs(host, ".discord.com")      || host == "discord.com"      ||
      dnsDomainIs(host, ".discord.gg")       || host == "discord.gg"       ||
      dnsDomainIs(host, ".discord.media")    ||
      dnsDomainIs(host, ".discordapp.com")   || host == "discordapp.com")
    return "SOCKS5 127.0.0.1:{porta_socks}";
  return "DIRECT";
}}
"#
    )
}

pub async fn servir(porta: u16, porta_socks: u16) -> Result<()> {
    let escuta = TcpListener::bind(("127.0.0.1", porta)).await?;
    crate::socks::log::linha(&format!("PAC servido em http://127.0.0.1:{porta}/proxy.pac"));
    let corpo = texto(porta_socks);

    loop {
        let (conexao, _) = escuta.accept().await?;
        let corpo = corpo.clone();
        tokio::spawn(async move {
            let _ = responder(conexao, corpo).await;
        });
    }
}

async fn responder(mut c: TcpStream, corpo: String) -> Result<()> {
    let mut lixo = [0u8; 1024];
    let _ = c.read(&mut lixo).await;
    let resp = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/x-ns-proxy-autoconfig\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n{}",
        corpo.len(),
        corpo
    );
    c.write_all(resp.as_bytes()).await?;
    c.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pac_manda_discord_pro_proxy_e_o_resto_direto() {
        let p = texto(9250);
        assert!(p.contains("SOCKS5 127.0.0.1:9250"));
        assert!(p.contains(".discord.com"));
        assert!(p.contains(".discord.media"));
        assert!(p.trim_end().ends_with('}'));
        assert!(p.contains("return \"DIRECT\""));
    }
}

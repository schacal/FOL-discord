# desbuga-discord

Corrige o bug do Discord que derruba a **transmissão de tela** em vários provedores brasileiros.

## Instalar

Abra o PowerShell e cole:

```powershell
irm https://raw.githubusercontent.com/schacal/desbuga-discord/main/install.ps1 | iex
```

Feche e abra o Discord uma vez. Pronto — funciona em todo reinício, sozinho, até você desinstalar.

Não precisa de administrador. Não precisa instalar Python, .NET, VPN nem conta em lugar nenhum.

## Desinstalar

```powershell
& "$env:LOCALAPPDATA\DesbugaDiscord\desbuga-discord.exe" desinstalar
```

Devolve o proxy automático do Windows ao valor anterior e apaga a pasta. Sem rastro.

## O que está acontecendo

O Discord decide a região da sua sessão pelo IP que enxerga **no momento em que abre**. Em provedores brasileiros com peering ruim essa decisão sai errada, e o sintoma mais visível é a transmissão de tela parar de funcionar.

A correção é a mesma que já se fazia na mão — abrir o Discord com uma VPN ligada e desligar depois. Este programa faz isso sozinho, sem VPN, e só com as conexões que importam:

| Sai por um IP estrangeiro | Sai direto, como sempre |
| --- | --- |
| `discord.com` | Áudio e vídeo das chamadas (UDP) |
| `gateway.discord.gg` | Transmissão de tela |
| `latency.discord.media` | `cdn.discordapp.com` e imagens |
| | Todo o resto da internet |

São 14 conexões e alguns kilobytes na abertura. **O ping não muda** — a voz e a tela nunca passam pelo exterior, e o servidor de voz que você recebe continua sendo o brasileiro.

## Como funciona por dentro

1. Um proxy SOCKS5 roda em `127.0.0.1:9250`, invisível, e sobe com o Windows.
2. Ele mantém uma piscina de proxies públicos gratuitos, validando cada um contra o próprio Discord: está de pé? fala com o Discord? tira você da região `brazil`? Quem falha é trocado sozinho.
3. Um arquivo PAC em `127.0.0.1:9251` diz ao Windows para entregar só o tráfego do Discord a esse proxy. O resto da internet nem passa por ali.
4. O Discord obedece ao proxy automático do Windows em toda abertura — por isso não é preciso mexer em atalho, e a correção sobrevive às atualizações dele.

### Suas credenciais estão seguras

O proxy carrega tráfego **HTTPS**, em túnel. Quem opera o proxy vê apenas *que* você falou com `discord.com` — nunca o conteúdo, nunca seu token, nunca suas mensagens. O programa não instala certificado raiz e não seria capaz de ler nada mesmo que quisesse.

## Comandos

```powershell
desbuga-discord instalar      # liga a correção e faz subir com o Windows
desbuga-discord status        # mostra o estado atual
desbuga-discord desinstalar   # remove tudo
desbuga-discord rodar         # roda em primeiro plano, para depurar
```

Se a correção padrão não bastar na sua máquina, existe uma rede de segurança que manda **todo** domínio do Discord pelo exterior:

```powershell
desbuga-discord rodar --tudo-discord
```

Log em `%LOCALAPPDATA%\DesbugaDiscord\desbuga.log`.

## Compilar do código-fonte

```bash
cargo build --release
```

O binário sai em `target/release/desbuga-discord.exe`. Os releases publicados são compilados pelo GitHub Actions a partir deste mesmo repositório.

## Licença

MIT

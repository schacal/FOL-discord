<h1 align="center">FOL-discord</h1>

<p align="center">
  <i><b>F</b>az <b>O</b> <b>L</b>ink sair do país — e voltar.</i>
</p>

<p align="center">
  Devolve a <b>transmissão de tela</b> e a <b>câmera</b> do Discord no Brasil.<br>
  Sem VPN, sem conta, sem mensalidade, sem administrador, sem perder ping. Instala e esquece.
</p>

<p align="center">
  <a href="https://github.com/schacal/FOL-discord/releases/latest"><img alt="release" src="https://img.shields.io/github/v/release/schacal/FOL-discord?style=flat-square"></a>
  <a href="https://github.com/schacal/FOL-discord/actions"><img alt="build" src="https://img.shields.io/github/actions/workflow/status/schacal/FOL-discord/release.yml?style=flat-square"></a>
  <img alt="tamanho" src="https://img.shields.io/badge/tamanho-1.8%20MB-blue?style=flat-square">
  <img alt="licença" src="https://img.shields.io/badge/licen%C3%A7a-MIT-green?style=flat-square">
</p>

---

## Instalar

Abra o **PowerShell** e cole:

```powershell
irm https://raw.githubusercontent.com/schacal/FOL-discord/main/install.ps1 | iex
```

Feche e abra o Discord uma vez. Pronto.

A partir daí funciona sozinho — em todo reinício do PC, em toda abertura do Discord, e nas reconexões que ele faz sozinho no meio do uso. Até você desinstalar.

**Não precisa** de administrador, VPN, conta, Python, .NET ou qualquer outra instalação.

## Desinstalar

```powershell
& "$env:LOCALAPPDATA\FolDiscord\fol-discord.exe" desinstalar
```

Devolve o proxy automático do Windows ao valor anterior e apaga a pasta. Sem rastro.

## Contexto

O Discord vem enfrentando pendências jurídicas no Brasil. Não é assunto deste repositório e não vamos entrar no mérito.

O que interessa aqui é o efeito prático que muita gente passou a sentir: **a transmissão de tela e a câmera pararam de funcionar.**

Este programa contorna esse efeito de forma rápida, sem perda de ping e sem abrir mão da sua segurança. O mecanismo é o mesmo que já se fazia na mão — abrir o Discord com uma VPN ligada e desligar depois — só que automático, permanente e sem VPN nenhuma.

## O que ele resolve

O Discord decide a região da sua sessão pelo IP que enxerga **no momento em que abre**. Essa decisão fica valendo para a sessão inteira, e quando ela sai errada a transmissão de tela e a câmera param.

A gambiarra conhecida era abrir o Discord com VPN ligada e desligar depois. Isto faz o mesmo, sozinho, sem VPN, e só com as conexões que realmente decidem a região.

| Sai por um IP estrangeiro | Sai direto, como sempre |
| --- | --- |
| `discord.com` | **Áudio, câmera e transmissão de tela** |
| `gateway.discord.gg` | `cdn.discordapp.com` e imagens |
| `latency.discord.media` | Todo o resto da internet |

São 14 conexões e alguns kilobytes na abertura.

**O ping não muda.** A voz e a tela viajam em UDP e nunca passam pelo proxy — o proxy do Windows só afeta TCP. E o servidor de voz que você recebe continua sendo o brasileiro: nos testes, `c-gru17` e `c-gru18`, ou seja, São Paulo.

## Comandos

```powershell
fol-discord instalar      # liga a correção e faz subir com o Windows
fol-discord status        # mostra o estado atual
fol-discord desinstalar   # remove tudo
fol-discord rodar         # roda em primeiro plano, para depurar
```

Se a correção padrão não bastar na sua máquina, existe uma rede de segurança que manda **todo** domínio do Discord pelo exterior:

```powershell
fol-discord instalar --tudo-discord
```

## Documentação

| | |
| --- | --- |
| [**Como funciona**](docs/como-funciona.md) | O mecanismo por dentro, as quatro peças, e os caminhos que não funcionaram |
| [**Segurança**](docs/seguranca.md) | O que está protegido, o que fica exposto, e como verificar por conta própria |
| [**Problemas**](docs/problemas.md) | Quando não funciona, e o que olhar |

**Resumo de segurança:** o tráfego é HTTPS em túnel, então quem opera o proxy vê apenas *que* você falou com o Discord — nunca o conteúdo, nunca seu token, nunca suas mensagens. Nenhum certificado raiz é instalado, nenhum arquivo do Discord é modificado, e nada roda como administrador. A [página de segurança](docs/seguranca.md) explica também o que **fica** exposto, porque não é zero.

## Estrutura

```
fol-discord/
├── src/
│   ├── main.rs        instalação, desinstalação, status, laço principal
│   ├── routing.rs     decide, por host, quem sai por fora
│   ├── socks.rs       o proxy local em 127.0.0.1:9250
│   ├── pool.rs        piscina de proxies públicos, com auto-cura
│   ├── pac.rs         o arquivo PAC que o Windows lê
│   └── windows.rs     as duas chaves de registro, e como desfazê-las
├── docs/
├── install.ps1
└── .github/workflows/release.yml
```

## Compilar

```bash
cargo build --release
```

Sai em `target/release/fol-discord.exe`. Os binários publicados em *Releases* são compilados pelo GitHub Actions a partir deste repositório — não são enviados da máquina de ninguém.

```bash
cargo test
```

## Por que FOL?

Oficialmente: **F**az **O** **L**ink sair do país e voltar. Sai, dá a volta, volta — e aí o Discord funciona.

Extraoficialmente: sim, é **Faz o L**.

As duas leituras estão corretas e nenhuma das duas foi acidente.

## Licença

MIT. Veja [LICENSE](LICENSE).

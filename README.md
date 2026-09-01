<p align="center">
  <img src="assets/banner.png" alt="FOL-discord — Faz O Link" width="100%">
</p>

<p align="center">
  <b>Devolve a transmissão de tela e a câmera do Discord no Brasil.</b><br>
  Sem VPN, sem conta, sem mensalidade, sem administrador, sem perder ping.
</p>

<p align="center">
  <a href="https://github.com/schacal/FOL-discord/releases/latest/download/FOL-discord-setup.exe">
    <img alt="Baixar o instalador para Windows" src="https://img.shields.io/badge/Baixar%20para%20Windows-instalador%20.exe-2ea44f?style=for-the-badge&logo=windows&logoColor=white">
  </a>
</p>

<p align="center">
  <sub>Windows 10 e 11, 64 bits. O download começa na hora, sempre na versão mais recente.</sub>
</p>

<p align="center">
  <img alt="versão" src="https://img.shields.io/github/v/release/schacal/FOL-discord?style=flat-square&label=vers%C3%A3o&cacheSeconds=1800">
  <img alt="build" src="https://img.shields.io/github/actions/workflow/status/schacal/FOL-discord/release.yml?style=flat-square&label=build">
  <img alt="licença" src="https://img.shields.io/badge/licen%C3%A7a-MIT-green?style=flat-square">
</p>

<p align="center">
  <img src="assets/demo.gif" alt="Janela do FOL-discord alternando entre funcionamento e pausa" width="760">
</p>

---

## Instalar

1. [**Baixe o `FOL-discord-setup.exe`**](https://github.com/schacal/FOL-discord/releases/latest/download/FOL-discord-setup.exe)
2. Abra o arquivo e siga o instalador
3. Pronto — a janela abre sozinha e a correção já está ligada

Depois disso ele se vira sozinho: a cada reinício do PC, a cada abertura do
Discord, e nas reconexões durante o uso.

> **O Windows vai avisar, e o antivírus pode reclamar.** É esperado e está
> explicado em [Antivírus e SmartScreen](#antivírus-e-smartscreen), com o
> relatório do VirusTotal e como conferir o arquivo antes de abrir.

**Não precisa** de administrador, VPN, conta, Python ou .NET.

<details>
<summary>Instalar pelo terminal, sem assistente</summary>

<br>

```powershell
irm https://raw.githubusercontent.com/schacal/FOL-discord/main/install.ps1 | iex
```

Baixa o mesmo instalador da versão mais recente, roda em modo silencioso e abre
a janela no fim. Serve para quem prefere o terminal ou instala em vários PCs.

</details>

## O que ele faz

O Discord decide a região da sua sessão pelo IP que enxerga **no momento em que
abre**, e essa decisão vale para a sessão inteira. Quando ela sai errada, a
transmissão de tela e a câmera param.

A gambiarra conhecida era abrir o Discord com VPN e desligar depois. Isto faz o
mesmo, sozinho, sem VPN, e só nas conexões que decidem a região:

| Sai por um IP estrangeiro | Sai direto, como sempre |
| --- | --- |
| `discord.com` | **Áudio, câmera e transmissão de tela** |
| `gateway.discord.gg` | `cdn.discordapp.com` e imagens |
| `latency.discord.media` | Todo o resto da internet |

São 14 conexões e alguns kilobytes na abertura.

**De onde vem o IP estrangeiro:** de proxies SOCKS5 públicos, de listas abertas,
operados por gente que ninguém conhece. O programa testa cada candidato e só usa
quem funciona. O tráfego é HTTPS em túnel — quem opera o proxy vê *que* você
falou com o Discord, nunca *o quê*. Vale ler [a página de
segurança](docs/seguranca.md) antes de instalar: ela é honesta sobre o que fica
exposto.

**O ping não muda.** Voz e tela viajam em UDP e nunca passam pelo proxy — o
proxy do Windows só afeta TCP. O servidor de voz continua sendo o brasileiro.

## A janela

Fechar a janela só a esconde na bandeja; a correção continua rodando.

| Controle | O que faz |
| --- | --- |
| **Pausar / Retomar** | desliga ou religa a correção, sem desinstalar nada |
| **Verificar agora** | confere o estado e religa o serviço se ele tiver parado |
| **Reiniciar Discord** | fecha e abre só o Discord |
| **Iniciar com o PC** | liga ou desliga a abertura automática no logon |
| **Desinstalar** | abre o desinstalador do Windows e desfaz tudo |

A janela avisa quando sai uma versão nova e abre o download. Ela nunca instala
nada por conta própria.

## Desinstalar

Botão **Desinstalar** dentro da janela, ou **FOL-discord** em *Aplicativos
instalados* do Windows. Pelo terminal: `fol-discord desinstalar`.

Devolve o proxy do Windows ao valor anterior, sai do PATH e apaga a pasta. Sem
rastro, e sem tocar em nenhum arquivo do Discord.

## Comandos

O instalador coloca o programa no PATH. **Abra um terminal novo:**

```powershell
fol-discord status              # o estado atual
fol-discord reiniciar-discord   # fecha e abre só o Discord
fol-discord desinstalar         # remove tudo
```

<details>
<summary>Mais comandos e opções</summary>

<br>

```powershell
fol-discord instalar                   # liga a correção e reinicia o Discord
fol-discord rodar                      # primeiro plano, para depurar
fol-discord instalar --sem-reiniciar   # não mexe no Discord aberto
fol-discord instalar --tudo-discord    # manda todo domínio do Discord pelo exterior
```

A última é rede de segurança, caso a correção padrão não baste na sua máquina.

Se o terminal disser `não é reconhecido como nome de cmdlet`, ele está com o
PATH antigo — abra uma janela nova, ou use o caminho completo:

```powershell
& "$env:LOCALAPPDATA\FolDiscord\fol-discord.exe" status
```

</details>

## Segurança

O tráfego é HTTPS em túnel: quem opera o proxy vê **que** você falou com o
Discord — nunca o conteúdo, o token ou as mensagens. Nenhum certificado raiz é
instalado, nenhum arquivo do Discord é modificado, nada roda como administrador.

A [página de segurança](docs/seguranca.md) explica o que **fica** exposto,
porque não é zero.

## Antivírus e SmartScreen

O instalador **não tem assinatura de código** — o projeto ainda não tem
certificado. Por isso duas coisas acontecem e as duas são esperadas:

**O SmartScreen mostra "Editor desconhecido".** Clique em *Mais informações* →
*Executar assim mesmo*, depois de conferir o arquivo (abaixo).

**Alguns antivírus marcam por heurística.** No
[relatório do VirusTotal](https://www.virustotal.com/gui/file/e05c1518783a301a446b992af6430fbe3451c4f407a65aa58fb857f555a7300c),
4 dos 71 motores acusam — todos com veredito genérico de aprendizado de máquina
(`Wacatac.B!ml`, `Unsafe`, `Malicious`), nenhum com assinatura de família real.
Kaspersky, ESET, BitDefender, Sophos, Symantec, Avast, Malwarebytes, TrendMicro,
Fortinet, McAfee e CrowdStrike passam limpo.

O motivo é o que o programa faz: mexe no proxy do Windows, sobe com o PC e busca
listas de proxies na internet. É a mesma descrição de um sequestrador de tráfego
— a diferença é que aqui o código está inteiro à vista.

### Conferindo o arquivo antes de abrir

```powershell
Get-FileHash "$env:USERPROFILE\Downloads\FOL-discord-setup.exe" -Algorithm SHA256
```

Compare com o `SHA256SUMS.txt` publicado na
[página da release](https://github.com/schacal/FOL-discord/releases/latest).

Quem tem o [GitHub CLI](https://cli.github.com) pode ir além e provar que o
arquivo saiu deste repositório, e não da máquina de alguém:

```powershell
gh attestation verify "$env:USERPROFILE\Downloads\FOL-discord-setup.exe" --repo schacal/FOL-discord
```

Se o seu antivírus bloquear, [reporte como falso
positivo](docs/problemas.md#o-antivírus-reclamou-do-executável) — é o único jeito
de tirar a detecção do banco do fabricante, e ajuda todo mundo que baixa depois.

## Documentação

| | |
| --- | --- |
| [**Como funciona**](docs/como-funciona.md) | O mecanismo por dentro e os caminhos que não funcionaram |
| [**Segurança**](docs/seguranca.md) | O que está protegido, o que fica exposto, como verificar |
| [**Problemas**](docs/problemas.md) | Quando não funciona, e o que olhar |
| [**Desenvolvimento**](docs/desenvolvimento.md) | Estrutura do repositório, compilar e testar |

## Por que FOL?

Oficialmente: **F**az **O** **L**ink sair do país e voltar. Sai, dá a volta,
volta — e aí o Discord funciona.

Extraoficialmente: sim, é **Faz o L**. As duas leituras estão corretas e nenhuma
foi acidente.

---

MIT. Veja [LICENSE](LICENSE).

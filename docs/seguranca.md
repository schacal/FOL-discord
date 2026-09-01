# Segurança

Este programa encaminha parte do seu tráfego do Discord por proxies públicos operados por desconhecidos. Isso merece uma explicação honesta, e não só a garantia de que está tudo bem. Abaixo está o que está protegido, o que fica exposto, e o que o programa deliberadamente não faz.

## Suas credenciais estão protegidas

**O que trafega é HTTPS, em túnel.** O proxy recebe um comando `CONNECT` e passa bytes cifrados de um lado para o outro. Ele não participa do TLS e não tem como participar: a validação do certificado acontece entre o Discord e o seu computador.

Concretamente, quem opera o proxy **não consegue ver**:

- seu token de sessão ou sua senha;
- suas mensagens, DMs ou nomes de servidores;
- qualquer conteúdo de qualquer requisição.

**O programa não instala certificado raiz.** É o único jeito de um intermediário ler HTTPS, e ele não faz isso — nem precisaria, nem seria capaz depois. Se algum dia algum instalador de "correção do Discord" pedir para instalar um certificado, é exatamente esse o momento de recusar.

## O que fica exposto — e isto é real

Não é nada, mas também não é zero:

**O operador do proxy vê seu endereço IP.** Ele sabe que um IP brasileiro passou por ele.

**Ele vê os nomes dos domínios.** O SNI do TLS viaja em claro, então ele sabe que você falou com `discord.com`. Sabe *que* você usa Discord — não sabe nada sobre *o que* você faz lá.

**Ele vê tamanho e ritmo do tráfego.** São alguns kilobytes na abertura da sessão.

**Ele pode derrubar a conexão, não lê-la.** Um proxy hostil consegue recusar ou cortar. O programa trata isso: rebaixa quem falha e, se não houver nenhum saudável, entrega a conexão direto. O pior caso é você ficar sem a correção — nunca sem Discord.

Vale a comparação: seu provedor de internet já vê exatamente essas mesmas três coisas, o tempo todo, em todo o seu tráfego. Aqui o alcance é menor — três domínios, alguns segundos por sessão.

## As listas de proxies são conteúdo não confiável

O programa busca listas públicas de IPs em repositórios do GitHub e no ProxyScrape. Qualquer uma dessas fontes pode, em tese, ser comprometida e servir proxies controlados por um atacante.

Isso não muda o quadro acima: **um proxy malicioso continua sem conseguir decifrar TLS.** O teto do estrago é o mesmo — ver seu IP, ver os domínios, ou derrubar a conexão.

Os candidatos ainda passam por uma validação antes de entrar em uso: precisam responder, falar com o Discord e efetivamente tirar você da região `brazil`.

## Risco na sua conta do Discord

Um risco real e que não é técnico: **o Discord pode notar a mudança de país e pedir verificação.** É o mesmo comportamento de quando alguém abre o Discord viajando.

Isso não aconteceu nos testes, mas é possível. Se o Discord pedir confirmação por e-mail ou telefone, é essa a causa. Você pode desinstalar a qualquer momento e voltar ao IP normal com um comando.

## O que fica no seu computador

As alterações do FOL são por usuário, sem administrador e reversíveis:

| O quê | Onde | Na desinstalação |
| --- | --- | --- |
| Proxy automático | `HKCU\...\Internet Settings\AutoConfigURL` | volta ao valor anterior, que é guardado antes de trocar |
| Autostart da interface | `Task Scheduler\FolDiscord.Bandeja` | removido somente se a ação for a interface instalada do FOL |
| Run legado do CLI | `HKCU\...\CurrentVersion\Run\FolDiscord` | removido somente quando aponta exatamente para o serviço do FOL |
| Entrada no PATH | `HKCU\Environment\Path` | só a nossa entrada é retirada |
| Executável e log | `%LOCALAPPDATA%\FolDiscord\` | pasta apagada |

O PATH é lido e gravado como valor bruto, preservando o tipo `REG_EXPAND_SZ`. Isso importa: reescrevê-lo como texto simples congelaria variáveis como `%USERPROFILE%\bin` que já estivessem lá, quebrando o PATH de quem instalou.

Nada é escrito em `HKLM`, em `Arquivos de Programas` ou no diretório do Discord. A tarefa da bandeja é interativa, limitada e só existe após o logon; ela não usa SYSTEM, elevação ou UAC. **Nenhum arquivo do Discord é modificado** — nem `settings.json`, nem atalhos, nem os binários. Por isso atualizações do Discord não quebram nada e não há o que restaurar.

## As portas locais não estão expostas

Os servidores escutam em `127.0.0.1` — não em `0.0.0.0`. Ninguém na sua rede local, e muito menos na internet, alcança as portas 9250 ou 9251. Só programas rodando na sua própria máquina, que de todo modo já poderiam abrir conexões por conta própria.

| Porta | O quê |
| --- | --- |
| 9250 | o proxy SOCKS5 local |
| 9251 | o arquivo PAC que o Windows lê |

A janela não abre uma terceira porta: ela conversa com o processo Tauri pelo IPC
local do próprio aplicativo. Não há API HTTP de controle em `127.0.0.1:9252`.

## A janela não é uma porta de saída

A janela distribuída é um WebView2 com **política de conteúdo restrita**. Ela
não se conecta a uma API HTTP local: usa apenas IPC do Tauri para pedir ações
ao processo nativo. Fonte, scripts e imagens já vêm no executável; o serviço
**não** — ele é instalado ao lado da janela pelo próprio setup, e é de lá que ela
o copia. Carregar um executável inteiro como dado dentro de outro e gravá-lo em
disco na execução é o desenho que os antivírus leem como conta-gotas, e era
desnecessário: o instalador já entrega o arquivo.

Duas consequências que valem dizer em voz alta:

- **A janela consulta somente a última release pública do GitHub.** Ao abrir,
  cada vez que é trazida da bandeja para a frente (com folga mínima de dez
  minutos entre consultas) e, de resto, uma vez a cada seis horas. A consulta
  envia a versão do FOL no `User-Agent` e recebe os metadados públicos da
  release; não envia atividade do Discord, proxies, conta, nem outro dado da
  pessoa. Só aparece aviso se houver uma release estável mais nova com o
  instalador oficial dentro da própria release.
- **A janela não baixa nem instala atualização sozinha.** Ao clicar no aviso,
  ela abre no navegador o download oficial do setup; a pessoa ainda escolhe se
  quer executar o instalador.
- **A janela usa uma ponte nativa local.** Essa ponte lê o estado do registro e
  as linhas de rota do `fol.log`, e aplica as ações da interface. Nada desse
  conteúdo sai do computador.

As dependências de terceiros dela são poucas e todas MIT, e o `.exe` publicado sai do GitHub Actions como o do serviço.

## O que o programa não faz

- Não lê nem toca no seu token do Discord.
- Não modifica arquivo nenhum do Discord. Ele **fecha e reabre** o Discord na instalação, porque a correção só vale a partir da próxima abertura — mas não altera nada dentro dele. Use `--sem-reiniciar` se preferir fazer isso na mão.
- Não injeta código no Discord (nada de client mod, nada de BetterDiscord).
- Não coleta telemetria. A única consulta adicional é a release pública do
  GitHub, usada exclusivamente para verificar se existe um instalador novo.
- Não se atualiza sozinho. O binário só muda depois de a pessoa baixar e abrir
  o instalador indicado pela própria janela.
- Não pede administrador, e recusar-se a dá-lo não muda nada.

## Verificando por conta própria

O código é curto o bastante para ser lido inteiro — cerca de 1.600 linhas em oito arquivos no serviço. Os pontos que valem conferir:

| O que verificar | Onde |
| --- | --- |
| Que só três domínios saem por fora | [`src/routing.rs`](../src/routing.rs) |
| Que as portas escutam só em `127.0.0.1` | [`src/socks.rs`](../src/socks.rs), [`src/pac.rs`](../src/pac.rs) |
| Que nada além do PAC e do autostart é escrito | [`src/windows.rs`](../src/windows.rs) |
| Que nenhum certificado é instalado | qualquer arquivo — não existe esse código |
| Que a janela não abre API HTTP de controle | [`interface/src-tauri/src/servico.rs`](../interface/src-tauri/src/servico.rs) e [`interface/src/api/tauri.ts`](../interface/src/api/tauri.ts) |
| Que a janela não pede nada além do necessário | [`interface/src-tauri/capabilities/default.json`](../interface/src-tauri/capabilities/default.json) |

Para ver ao vivo por onde cada conexão saiu:

```powershell
Get-Content "$env:LOCALAPPDATA\FolDiscord\fol.log" -Tail 30 -Encoding utf8
```

Cada linha diz `exterior` ou `direto` e o destino.

Os binários publicados em *Releases* são compilados pelo GitHub Actions a partir deste repositório, em execução pública e auditável — não são enviados da máquina de ninguém.

## Conferindo o instalador que você baixou

O instalador **ainda não tem assinatura de código** — o projeto não tem
certificado. Enquanto não tiver, sobram duas provas, e as duas são verificáveis
por você, sem confiar na nossa palavra.

**A soma SHA-256.** Cada release publica um `SHA256SUMS.txt` ao lado do
instalador:

```powershell
Get-FileHash "$env:USERPROFILE\Downloads\FOL-discord-setup.exe" -Algorithm SHA256
```

**O atestado de procedência.** Mais forte que a soma: ele é assinado pela
infraestrutura do próprio GitHub e amarra o arquivo ao commit e à execução do
workflow que o produziu. Prova que o binário saiu deste repositório, e não da
máquina de alguém.

```powershell
gh attestation verify "$env:USERPROFILE\Downloads\FOL-discord-setup.exe" --repo schacal/FOL-discord
```

Quando existir certificado, esta conferência entra também:

```powershell
Get-AuthenticodeSignature "$env:USERPROFILE\Downloads\FOL-discord-setup.exe" | Format-List Status, SignerCertificate
```

O `install.ps1` já faz as duas primeiras sozinho: compara a soma do que baixou
com a publicada na release e recusa executar o arquivo se divergir.

## Por que não tem assinatura de código

Porque assinatura de confiança pública custa dinheiro e o projeto não tem
receita nenhuma. As rotas conhecidas são um certificado anual pago ou o
Microsoft Artifact Signing, que é mensal — e que, para pessoa física, só aceita
quem mora nos Estados Unidos ou no Canadá. A rota gratuita para software livre
é a [SignPath Foundation](https://signpath.org/), que exige inscrição e revisão
humana; o repositório já atende à parte técnica. Os detalhes estão em
[Desenvolvimento](desenvolvimento.md#assinatura-do-instalador).

Assinatura, vale dizer, não prova que um programa é inofensivo. Prova quem o
publicou. O atestado de procedência do GitHub prova a mesma coisa por outro
caminho — que o arquivo saiu deste repositório, deste commit, desta execução —
e é o que temos hoje.

## Se o antivírus acusar

Acontece, e é esperado num programa sem assinatura que troca o proxy do Windows.
O que cada motor diz, por que diz, e onde reportar como falso positivo está em
[Problemas](problemas.md#o-antivírus-reclamou-do-executável).

O que estava ao nosso alcance sem certificado já foi feito, a partir da v0.2.6:

- **Nada de executável escondido dentro de outro.** A janela não carrega mais o
  serviço como dado para gravá-lo em disco ao abrir — o instalador o entrega ao
  lado dela. Era o desenho clássico de *dropper*, e o motivo mais provável dos
  vereditos genéricos.
- **Nada de `tasklist` nem `taskkill`.** Achar e encerrar processos é feito
  pela API do Windows, não por utilitários de linha de comando com console
  escondida — que é o comportamento que as sandboxes dos antivírus pontuam.
- **Os dois executáveis se identificam.** Nome do produto, descrição,
  fabricante, direitos autorais e ícone estão nos recursos de versão do serviço
  e da janela; um `.exe` sem nada disso é o perfil que os modelos de reputação
  marcam antes de olhar o que ele faz.
- **As requisições se identificam.** As listas de proxies são buscadas com
  `User-Agent` do FOL-discord, não em branco.

O que sobra são vereditos de aprendizado de máquina (`!ml`, `Suspicious`,
`Malicious`), sem assinatura de família — e esses só saem do banco do
fabricante quando alguém submete a amostra pelo canal de falso positivo.
Reportar é o que resolve.

## Encontrou um problema de segurança?

Abra uma issue. Se for algo que exponha usuários, escreva sem detalhes de exploração e peça contato privado primeiro.

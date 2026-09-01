# Solução de problemas

Antes de qualquer coisa, veja o estado:

```powershell
fol-discord status
```

O esperado é `sim` em todas as linhas. Cada `não` aponta para uma seção abaixo.

## `fol-discord não é reconhecido como nome de cmdlet`

O terminal que você está usando foi aberto **antes** da instalação e ainda carrega o PATH antigo. Duas saídas:

- **abra uma janela nova** do PowerShell — a mais simples;
- ou use o caminho completo, que funciona em qualquer janela:

```powershell
& "$env:LOCALAPPDATA\FolDiscord\fol-discord.exe" status
```

Se o caminho completo também falhar dizendo que não encontrou o arquivo, então a instalação não chegou a acontecer. Rode:

```powershell
irm https://raw.githubusercontent.com/schacal/FOL-discord/main/install.ps1 | iex
```

## A transmissão de tela continua quebrada

Feche e abra o Discord uma vez. A correção vale a partir da próxima abertura, não na sessão que já estava aberta.

Se continuar, veja por onde as conexões saíram:

```powershell
Get-Content "$env:LOCALAPPDATA\FolDiscord\fol.log" -Tail 30 -Encoding utf8
```

**Se aparecem linhas `exterior discord.com:443`** — o encaminhamento está funcionando e o problema é outro. Vale tentar a rede de segurança, que manda todo domínio do Discord por fora:

```powershell
fol-discord desinstalar
fol-discord instalar --tudo-discord
```

**Se aparece `exterior indisponível`** — a piscina secou. Veja a seção seguinte.

**Se não aparece linha nenhuma do Discord** — ele não está usando o PAC. Veja "O Discord ignora o proxy".

## `exterior indisponível` no log

Nenhum proxy público sobreviveu à validação. Acontece: são gratuitos e morrem o tempo todo. O programa tenta de novo a cada 5 minutos sozinho.

Enquanto isso ele entrega as conexões direto, então **o Discord continua funcionando** — só sem a correção.

Se persistir por muito tempo, reinicie o serviço:

```powershell
taskkill /F /IM fol-discord.exe
fol-discord instalar
```

## O Discord ignora o proxy

Confirme que a chave está no lugar:

```powershell
Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings" -Name AutoConfigURL
```

Deve responder `http://127.0.0.1:9251/proxy.pac`.

Confirme que o PAC está sendo servido:

```powershell
Invoke-RestMethod "http://127.0.0.1:9251/proxy.pac"
```

Deve devolver a função `FindProxyForURL`. Se der erro de conexão, o serviço não está rodando.

Uma VPN, um antivírus com "proteção de rede" ou outro programa de proxy podem sobrescrever o `AutoConfigURL`. Se algum deles estiver ativo, os dois disputam a mesma chave e o último a escrever ganha.

## O serviço não sobe sozinho no boot

```powershell
schtasks /query /tn FolDiscord.Bandeja /fo list /v
Get-Process fol-discord-janela, fol-discord -ErrorAction SilentlyContinue
```

Se a tarefa estiver ausente, abra o setup instalado e ligue **Iniciar com o PC**.
No boot normal a janela não aparece: o esperado é o ícone âmbar **FOL-discord —
preparando**, que troca para o estado real quando o serviço responde.

Se há a tarefa e `fol-discord-janela.exe`, mas ainda não há `fol-discord.exe`,
o serviço ainda está preparando proxies. Se o Discord foi aberto antes de um
proxy estar pronto, feche e abra o Discord uma vez depois de o ícone deixar de
mostrar preparação. Não edite chaves de atraso do Explorer, prioridade de
processo ou valores aleatórios do Registro para tentar contornar isso.

## O Discord pediu verificação de e-mail ou telefone

Comportamento esperado dele ao ver a sessão vindo de outro país — o mesmo de quando alguém abre o Discord viajando. Confirme normalmente. Se preferir não conviver com isso, desinstale.

## O antivírus reclamou do executável

Acontece, e não é surpresa. O instalador ainda **não tem assinatura de código**, e
o que o programa faz — trocar o proxy do Windows, subir com o PC, buscar listas
de proxies na internet — descreve, palavra por palavra, um sequestrador de
tráfego. A diferença é que aqui o código está inteiro à vista.

No [relatório do VirusTotal](https://www.virustotal.com/gui/file/7bef02110fd14a27b668139f5f97068ffed60231324687b379cb2898372e93db)
de um build da v0.2.6, 4 dos 70 motores acusam:

| Motor | Veredito | O que é |
| --- | --- | --- |
| Microsoft | `Trojan:Win32/Wacatac.B!ml` | modelo de aprendizado de máquina; o sufixo `!ml` diz isso |
| SecureAge | `Malicious` | aprendizado de máquina |
| Trapmine | `Suspicious.low.ml.score` | aprendizado de máquina, e o próprio nome diz "pontuação baixa" |
| Bkav Pro | `W32.Malware.…` | hash genérico |

Nenhum é assinatura de família real. Kaspersky, ESET, BitDefender, Sophos,
Symantec, Avast, AVG, Avira, Malwarebytes, TrendMicro, Fortinet, McAfee,
CrowdStrike, SentinelOne, Elastic, Google, Palo Alto e ClamAV passam limpo.

Da v0.2.5 para a v0.2.6 o que dependia do projeto foi feito — o serviço deixou
de vir escondido dentro da janela, `tasklist` e `taskkill` saíram, os
executáveis passaram a se identificar. A lista do que mudou e por quê está em
[Segurança](seguranca.md#se-o-antivírus-acusar). O que sobra é o que só uma
assinatura de código resolve, e [por que ela ainda não
existe](seguranca.md#por-que-não-tem-assinatura-de-código) também está lá.

Para ver o relatório **do arquivo exato que você baixou**, pegue a soma do
`SHA256SUMS.txt` da release e cole na busca do VirusTotal — ou envie o próprio
arquivo, que é público e não tem nada seu dentro. Se ninguém tiver enviado
ainda, a busca não acha nada; isso só quer dizer que você é o primeiro.

### O que você pode fazer

1. **Conferir o arquivo** pelo SHA-256 e pelo atestado de procedência — está em
   [Segurança](seguranca.md#conferindo-o-instalador-que-você-baixou).
2. **Compilar você mesmo** com `cargo build --release` e comparar.
3. **Ler o código** — são cerca de 1.600 linhas em oito arquivos no serviço.
4. **Reportar o falso positivo.** É o único caminho que tira a detecção do banco
   do fabricante, e vale para todo mundo que baixar depois.

### Se o Defender bloqueou o download ou apagou o arquivo

O SmartScreen e o Defender agem em momentos diferentes, e a saída é diferente:

- **No navegador**, "arquivo não baixado com frequência" ou "não é comum baixar
  este arquivo": clique em *Manter* → *Mostrar mais* → *Manter assim mesmo*.
  Isso é reputação de download, não detecção.
- **Ao abrir**, "O Windows protegeu o computador": *Mais informações* →
  *Executar assim mesmo*. Confira o SHA-256 antes.
- **O Defender colocou em quarentena**: abra *Segurança do Windows* →
  *Proteção contra vírus e ameaças* → *Histórico de proteção*, escolha a
  detecção e use *Ações* → *Permitir*. Depois baixe de novo.

### Onde reportar falso positivo

| Fabricante | Canal |
| --- | --- |
| Microsoft Defender | <https://www.microsoft.com/en-us/wdsi/filesubmission> — escolha *Software developer*, marque **Incorrectly detected as malware** |
| SecureAge | <https://www.secureage.com/support> |
| Bkav | <https://www.bkav.com/report-false-positive> |

Ao reportar, informe o link deste repositório e o do relatório do VirusTotal: os
dois juntos costumam bastar, porque mostram origem e código-fonte.

## A janela não abre

Abra o `fol-discord-janela.exe` instalado pelo setup. O instalador deixa o
serviço (`fol-discord.exe`) ao lado dela, e é de lá que a janela o instala — não
precisa abrir PowerShell antes. Copiar só a janela para outra pasta não funciona:
sem o serviço vizinho ela avisa e pede para reinstalar. Se não subir, estas são
as causas mais prováveis, nessa ordem.

**Falta o WebView2.** A janela usa o WebView2, que já vem no Windows 11 e na maioria dos Windows 10 atuais. Se ele faltar, instale o *Evergreen Bootstrapper* da Microsoft — a instalação por usuário não pede administrador.

**O serviço não está rodando.** A janela abre mesmo assim, no estado **Parado**.
Clique em **Verificar agora**; ele inicia uma única cópia do serviço e atualiza
o estado. Se não religar, confira se o executável do serviço está no lugar:

```powershell
Get-ChildItem "$env:LOCALAPPDATA\FolDiscord"
```

**A janela já está aberta, escondida na bandeja.** Fechar esconde, não encerra — é assim de propósito. Procure o ícone perto do relógio; ele carrega a cor do estado. Abrir uma segunda vez traz a primeira para a frente em vez de abrir outra.

Para sair de verdade, use **Sair (o serviço continua)** no menu da bandeja. Como o rótulo diz, isso fecha só a janela: a correção continua valendo.

**Janelas pretas aparecem na instalação ou remoção.** Use a versão mais nova
de `fol-discord-janela.exe`. Os processos auxiliares da janela e do serviço são
criados sem console; não deve aparecer CMD durante Instalar, Reiniciar Discord,
Verificar agora ou Desinstalar.

## O WSL avisou que o proxy do host mudou

Ao ligar ou desligar o PAC, o Windows muda a configuração de proxy do usuário.
Se o WSL estiver configurado para herdar esse proxy, ele pode mostrar um aviso
pedindo para reiniciar o WSL. Isso é esperado e não impede o FOL-discord de
funcionar.

Não é preciso fazer nada para o Discord. Se você usa WSL e quer que ele aplique
a mudança imediatamente, rode `wsl --shutdown` e abra sua distribuição de novo.
Se preferir que o WSL nunca herde proxies do Windows, desative `autoProxy` na
configuração global do WSL — essa escolha afeta todas as distribuições Linux,
não só este programa.

## Remover tudo

```powershell
fol-discord desinstalar
```

Devolve o `AutoConfigURL` ao valor anterior, remove o autostart e apaga a pasta. Feche e abra o Discord depois. Nenhum arquivo do Discord foi tocado em momento nenhum, então não há mais nada a restaurar.

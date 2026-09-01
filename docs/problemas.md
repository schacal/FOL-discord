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
Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name FolDiscord
```

Se não existir, rode `instalar` de novo. Se existir e mesmo assim não subir, algum otimizador de inicialização pode estar bloqueando entradas do `Run`.

## O Discord pediu verificação de e-mail ou telefone

Comportamento esperado dele ao ver a sessão vindo de outro país — o mesmo de quando alguém abre o Discord viajando. Confirme normalmente. Se preferir não conviver com isso, desinstale.

## O antivírus reclamou do executável

Binários Rust novos e sem assinatura de código costumam ser sinalizados por heurística, e um programa que abre um proxy local reforça isso. Você pode:

- compilar você mesmo com `cargo build --release` e comparar;
- conferir que o binário do *Release* saiu do GitHub Actions, cujo log de execução é público;
- ler o código — são cerca de 700 linhas.

## Remover tudo

```powershell
fol-discord desinstalar
```

Devolve o `AutoConfigURL` ao valor anterior, remove o autostart e apaga a pasta. Feche e abra o Discord depois. Nenhum arquivo do Discord foi tocado em momento nenhum, então não há mais nada a restaurar.

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

Duas marcas, ambas em `HKCU`, ambas sem administrador, ambas reversíveis:

| O quê | Onde | Na desinstalação |
| --- | --- | --- |
| Proxy automático | `HKCU\...\Internet Settings\AutoConfigURL` | volta ao valor anterior, que é guardado antes de trocar |
| Autostart | `HKCU\...\CurrentVersion\Run\DesbugaDiscord` | removido |
| Executável e log | `%LOCALAPPDATA%\DesbugaDiscord\` | pasta apagada |

Nada é escrito em `HKLM`, em `Arquivos de Programas` ou no diretório do Discord. **Nenhum arquivo do Discord é modificado** — nem `settings.json`, nem atalhos, nem os binários. Por isso atualizações do Discord não quebram nada e não há o que restaurar.

## As portas locais não estão expostas

Os dois servidores escutam em `127.0.0.1` — não em `0.0.0.0`. Ninguém na sua rede local, e muito menos na internet, alcança as portas 9250 ou 9251. Só programas rodando na sua própria máquina, que de todo modo já poderiam abrir conexões por conta própria.

## O que o programa não faz

- Não lê nem toca no seu token do Discord.
- Não modifica arquivo nenhum do Discord.
- Não injeta código no Discord (nada de client mod, nada de BetterDiscord).
- Não coleta telemetria e não envia nada para lugar nenhum além das listas de proxies e do próprio Discord.
- Não se atualiza sozinho. O binário só muda se você mandar.
- Não pede administrador, e recusar-se a dá-lo não muda nada.

## Verificando por conta própria

O código é curto o bastante para ser lido inteiro — cerca de 700 linhas em seis arquivos. Os pontos que valem conferir:

| O que verificar | Onde |
| --- | --- |
| Que só três domínios saem por fora | [`src/routing.rs`](../src/routing.rs) |
| Que as portas escutam só em `127.0.0.1` | [`src/socks.rs`](../src/socks.rs), [`src/pac.rs`](../src/pac.rs) |
| Que nada além do PAC e do autostart é escrito | [`src/windows.rs`](../src/windows.rs) |
| Que nenhum certificado é instalado | qualquer arquivo — não existe esse código |

Para ver ao vivo por onde cada conexão saiu:

```powershell
Get-Content "$env:LOCALAPPDATA\DesbugaDiscord\desbuga.log" -Tail 30
```

Cada linha diz `exterior` ou `direto` e o destino.

Os binários publicados em *Releases* são compilados pelo GitHub Actions a partir deste repositório, em execução pública e auditável — não são enviados da máquina de ninguém.

## Encontrou um problema de segurança?

Abra uma issue. Se for algo que exponha usuários, escreva sem detalhes de exploração e peça contato privado primeiro.

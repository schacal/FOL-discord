# Novidades

Cada versão publicada em [Releases](https://github.com/schacal/FOL-discord/releases)
e o que mudou nela. Datas no formato ano-mês-dia.

## 0.2.9 — 2026-09-02

**Na abertura, tudo o que é do Discord sai pelo exterior.** Até aqui só três
hosts saíam por fora — `discord.com`, o gateway e `latency.discord.media` — e
o resto do Discord ia direto desde o começo. Agora o programa faz o que a
correção manual sempre fez: liga a "VPN" para o Discord abrir e desliga
assim que a sessão nasceu. Enquanto a janela está aberta, API, gateway, CDN,
anexos e o TCP dos servidores de voz saem por um IP estrangeiro; quando ela
fecha, tudo isso é derrubado e o Discord reconecta direto, com a região já
gravada na sessão.

**A janela continua fechando cedo.** Só as conexões que decidem a região —
as mesmas três de antes — contam para o silêncio de 30 segundos. Se a CDN
também contasse, um Discord em uso nunca deixaria o silêncio completar e a
janela só fecharia pelo teto, dois minutos depois, no meio do uso. O que é
desviado e o que segura a janela são duas listas diferentes de propósito, e
há um teste que falha se elas voltarem a se misturar. Os 30 segundos
continuam sendo os da medição da 0.2.8.

**O que isso custa.** Durante esse primeiro minuto, tudo o que o Discord
carrega passa por um proxy público gratuito, e vem mais devagar. Começar uma
transmissão de tela nesse minuto pode falhar: a sinalização dela viaja pelo
gateway, que é derrubado quando a janela fecha. É inerente ao modelo —
desligar a VPN corta as conexões — e por isso a janela fecha o mais cedo que
dá. O `status.discord.com`, que a 0.2.8 tinha tirado do exterior, volta a
sair por fora junto com o resto: com a VPN ligada não há exceção, e ele
continua sem segurar a janela.

**Proxy mudo não prende mais o Discord.** O aperto de mão com o proxy
estrangeiro não tinha prazo: um que aceitasse a conexão e não respondesse
prendia a requisição para sempre. Agora o TCP e o aperto de mão juntos têm
5 segundos, com uma segunda tentativa noutro proxy — antes eram 15 segundos
só para o TCP, duas vezes, e o resto sem limite. O log de um usuário tinha 31
esperas estouradas. E se a janela fechar enquanto um aperto de mão ainda
corre, a conexão é aberta direto em vez de nascer no exterior só para cair
em seguida.

**Piscina magra reabastece na hora.** Quando os proxies em uso caem abaixo
de três, a manutenção acorda em vez de esperar a passada de cinco minutos.
Com tudo saindo pelo exterior, um proxy ruim é rebaixado em duas conexões, e
cinco minutos de piscina vazia seriam cinco minutos de janela aberta sem para
onde desviar.

**Reinício do Discord, detectado pelo processo certo.** O serviço passa a
identificar o Discord pelo processo principal — o único `Discord.exe` cujo
pai não é outro `Discord.exe` — e pela hora em que ele nasceu, porque o
Windows reaproveita PIDs depressa. Uma leitura vazia da lista de processos,
que acontece sob carga, não é mais lida como "o Discord reiniciou": são
precisas três seguidas para aceitar que ele fechou. Fechar o Discord agora
também aparece no log.

**Suspeita registrada, sem reiniciar nada.** Se um gateway novo abrir mais de
um minuto depois de o anterior cair, com a sessão já aberta — o PC dormiu, a
internet caiu —, o serviço escreve no log que a sessão pode ter renascido
pelo IP brasileiro. Só escreve: o programa não consegue ler a região da
sessão em curso, e reiniciar o Discord por palpite seria pior do que o
problema.

**`--tudo-discord` saiu.** Era a rede de segurança para mandar todo o
Discord pelo exterior, e nunca chegou a funcionar pelo caminho normal: o
instalador subia o serviço sem repassar a opção. Agora é o comportamento
padrão, sem opção nenhuma.

## 0.2.8 — 2026-09-02

**As mensagens voltaram a sair direto.** O desvio pelo exterior não tinha hora
para acabar: `discord.com` e `gateway.discord.gg` continuavam saindo por um
proxy público pela sessão inteira. Como são justamente o caminho das mensagens
— e o gateway é um websocket que fica de pé por horas —, cada mensagem pagava a
latência do proxy. Medido nesta máquina:

| | Direto | Pelo proxy |
| --- | --- | --- |
| `discord.com/api` | 0,06 s | 2,1 s |
| `gateway.discord.gg` | 0,20 s | 2,9 s a 11,3 s |

Também era isso o que fazia o Discord parecer reiniciar sozinho: o proxy
gratuito derrubava o websocket, o Discord reconectava do zero, e a reconexão
saía pelo mesmo caminho lento.

Agora o desvio tem janela. Ele vale enquanto a sessão está nascendo — que é o
único momento em que o IP é lido — e acaba 30 segundos depois da última conexão
de controle, com teto de 120 segundos. Ao fechar, o programa derruba o que ele
mesmo abriu pelo exterior, e o Discord reconecta direto com `RESUME`, mantendo a
sessão e a região. É o mesmo efeito de desligar a VPN depois de abrir o Discord.

Os 30 segundos saíram de cronometrar a abertura, não de chute: o Discord falou
com o `discord.com` aos 10 s, com o gateway aos 12 s e só voltou ao
`latency.discord.media` aos 43 s. Uma janela mais curta fechava no meio dessa
sequência.

A janela também não vence quando não tem o que corrigir: piscina ainda sem
proxy validado, Discord fechado, ou aperto de mão com o upstream ainda em voo.
Cada um desses casos já fez a correção se perder em teste.

**Reinício do Discord re-arma a correção.** O serviço compara os processos
`Discord.exe` a cada segundo e reabre a janela quando nenhum dos anteriores
sobrou — um Discord novo tem sessão nova, e a região dele ainda vai ser
decidida. Renderizador que nasce ou morre no meio do uso não conta. Fechar o
Discord também reabre a janela, para ela já estar de pé quando ele voltar.

**O que isso custa.** Enquanto a janela está aberta, o gateway passa pelo proxy
— mais ou menos um minuto depois de abrir o Discord, as mensagens vêm devagar.
Depois disso ele é derrubado, reconecta direto, e assim fica pelo resto da
sessão.

**`status.discord.com` parou de sair pelo exterior.** Casava com `discord.com` e
ia para o proxy sem comprar nada: é a página pública de avisos, não participa da
decisão de região.

A voz, a câmera e a transmissão de tela não mudaram — são UDP e nunca passaram
pelo proxy.

## 0.2.7 — 2026-09-01

**Atualização.** A janela consultava a última release ao abrir e depois só de
seis em seis horas. Como ela passa o dia escondida na bandeja, quem clicava
nela logo depois de uma release nova não via aviso nenhum. Agora cada vez que
a janela é trazida para a frente — pelo ícone da bandeja ou por uma segunda
abertura — ela consulta de novo, com folga mínima de dez minutos entre
consultas. A consulta continua sendo só a release pública do GitHub; nada da
pessoa é enviado.

**Serviço trocado depois de atualizar por cima.** Quando o instalador novo
só substituía os arquivos (modo silencioso, ou **Não desinstalar** na tela do
instalador), o serviço antigo continuava rodando até o próximo logon. Agora a
janela percebe que a cópia em execução não é a que o instalador trouxe e a
troca sozinha, sem religar o proxy de quem tinha pausado.

**Limitação conhecida ao atualizar.** O instalador pergunta se deve
*desinstalar antes de instalar*, e essa é a opção marcada por padrão. Ela roda
o desinstalador da versão antiga, que devolve o proxy, apaga a tarefa de logon
e fecha o Discord — como faria numa remoção. Por enquanto, ao atualizar, marque
**Não desinstalar**. Se já passou pela primeira opção: a correção volta sozinha
quando a janela abre; **Iniciar com o PC** precisa ser religado na janela. Uma
guarda no desinstalador para reconhecer a atualização foi tentada nesta versão
e não segurou no teste, por isso ficou de fora.

## 0.2.6 — 2026-09-01

Versão de limpeza: menos motivos para o antivírus reclamar, e o aviso de versão
nova voltando a funcionar.

**Antivírus.** O que dependia do projeto, sem certificado, foi feito:

- O serviço deixou de vir embutido dentro da janela como dado e gravado em disco
  ao abrir — o instalador o entrega ao lado dela. Esse desenho é o que os
  antivírus classificam como *dropper*.
- `tasklist` e `taskkill` saíram do código. Processos são encontrados e
  encerrados pela API do Windows.
- O serviço passou a carregar nome do produto, descrição, fabricante, direitos
  autorais e ícone nos recursos de versão. A janela já carregava.
- As requisições das listas de proxies passaram a se identificar com
  `User-Agent`.
- A janela caiu de 7,1 MB para 4,8 MB.

**Atualização.** A v0.2.5 saiu só com o instalador de nome fixo, e a janela
procurava só o nome com versão — quem estava na v0.2.4 nunca viu o aviso. Agora:

- a release publica os dois nomes, e o workflow recusa publicar se a tag não
  bater com a versão dos manifestos;
- a janela aceita qualquer um dos dois, desde que esteja dentro da própria
  release deste repositório.

**Desinstalação.** O desinstalador do Windows deixou de duplicar a limpeza: ele
confere se a janela está aberta e delega tudo ao serviço, que já sabe restaurar
o proxy, validar e remover o autostart e fechar o Discord.

**Release.** Cada release publica a soma SHA-256 e um atestado de procedência
assinado pelo GitHub. O workflow assina o instalador quando houver credenciais,
e publica sem assinatura, avisando, quando não houver. A documentação explica
[por que ainda não há certificado](docs/seguranca.md#por-que-não-tem-assinatura-de-código)
e qual é a rota gratuita.

## 0.2.5 — 2026-09-01

- O instalador passou a ser o único jeito de instalar: setup NSIS por usuário,
  sem administrador, com a janela e o serviço juntos.
- **Desinstalar** dentro da janela passou a funcionar depois de uma instalação
  pelo setup.
- "Última checagem" passou a ser carimbada pelo serviço a cada manutenção da
  piscina, não só pelo botão **Verificar agora**.
- O ícone do programa passou a ser a logo ilustrada.

## 0.2.4 — 2026-09-01

- Primeira versão com instalador para Windows, ícone na bandeja e **Iniciar com
  o PC** por tarefa de logon, sem elevação.

## 0.2.0 a 0.2.3 — 2026-09-01

- A janela de gerenciamento em Tauri + React: estado, métricas, atividade,
  **Pausar / Retomar**, **Verificar agora** e **Reiniciar Discord**.
- Ajustes de empacotamento e de release entre uma e outra.

## 0.1.0 — 2026-09-01

- O serviço em linha de comando: proxy SOCKS5 local, PAC do Windows, piscina de
  proxies públicos com validação contra o próprio Discord, `instalar`,
  `desinstalar` e `status`.

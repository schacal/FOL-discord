# Novidades

Cada versão publicada em [Releases](https://github.com/schacal/FOL-discord/releases)
e o que mudou nela. Datas no formato ano-mês-dia.

## 0.2.7 — 2026-09-01

**Atualização.** A janela consultava a última release ao abrir e depois só de
seis em seis horas. Como ela passa o dia escondida na bandeja, quem clicava
nela logo depois de uma release nova não via aviso nenhum. Agora cada vez que
a janela é trazida para a frente — pelo ícone da bandeja ou por uma segunda
abertura — ela consulta de novo, com folga mínima de dez minutos entre
consultas. A consulta continua sendo só a release pública do GitHub; nada da
pessoa é enviado.

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

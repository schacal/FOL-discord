# Interface do FOL-discord

Janela única de gerenciamento, mais ícone de bandeja. Tauri v2 + React + Tailwind.

## Ver agora, sem Rust

```bash
npm install
npm run dev
```

Abre em `http://localhost:5173`. A janela aparece no tamanho real (750 × 475)
sobre um fundo neutro, falando com um **serviço simulado** — nenhum proxy, nenhum
registro, nenhuma porta.

A barra abaixo da janela força os cenários:

| Botão | O que testa |
| --- | --- |
| Serviço no ar | estado `operacional` |
| Piscina seca | estado `sem_proxies` |
| Serviço caído | estado `parado` |
| Atualização disponível | a pastilha de atualização no cabeçalho |
| Envelhecer checagem | "última checagem" com mais de uma hora |

O estado `pausado` sai clicando em **Pausar** — e as conexões novas passam a
sair `direto` na lista, que é o critério de aceite.

## Compilar o aplicativo

Precisa de Rust e do WebView2 (já vem no Windows 11).

```bash
npm run tauri dev      # janela de verdade, instalando o serviço embutido
npm run tauri build    # sai em src-tauri/target/release/fol-discord-janela.exe
```

O executável é autocontido: no build, ele compila a cópia estável de
`fol-discord.exe` e a embute junto ao frontend. Ao abrir, instala essa cópia
em `%LOCALAPPDATA%\FolDiscord`, a inicia em segundo plano e reinicia o Discord
uma vez na primeira instalação. Não depende de PowerShell, PATH, uma instalação
anterior ou de outro `.exe` ao lado.

## Ícones

```bash
node scripts/icones.mjs
npx tauri icon src-tauri/icones/fonte.png -o src-tauri/icones
```

O primeiro comando desenha a marca — o "L" que sai e volta — e as quatro cores
de bandeja. O segundo vira o conjunto de ícones do Windows. Só rode se mexer na
marca; os arquivos já estão no repositório.

## Como isto conversa com o serviço

O webview chama comandos nativos do Tauri pelo IPC do aplicativo. Não existe
API HTTP na porta 9252. A ponte nativa instala a cópia embutida, consulta o
processo e o registro do Windows e usa o núcleo de linha de comando apenas nas
ações que precisam dele (`instalar`, `reiniciar-discord` e `desinstalar`).

Para a atividade, ela lê somente as linhas `exterior` e `direto` do `fol.log`;
diagnósticos como `early eof` não aparecem como conexão. As consultas de estado
e todos os processos auxiliares são invisíveis para a pessoa usando o programa:
nada deve abrir uma janela de CMD.

```
src/api/tipos.ts      o contrato, copiado do handoff
src/api/tauri.ts      o cliente nativo de verdade
src/api/simulado.ts   o serviço de mentira, só no navegador
src/api/index.ts      escolhe um dos dois
```

Trocar de um para o outro não é configuração: dentro do Tauri é sempre o
serviço de verdade.

## Controles da janela instalada

| Controle | Resultado esperado |
| --- | --- |
| **Pausar / Retomar** | remove ou restaura o PAC do Windows, mantendo o serviço instalado |
| **Verificar agora** | religa o serviço se necessário e atualiza o estado; uma ação nunca inicia duas cópias |
| **Reiniciar Discord** | fecha e relança só o Discord; não espera a validação de proxies |
| **Iniciar com o PC** | cria ou remove `FolDiscord.Bandeja`, uma tarefa limitada do usuário que chama a interface instalada com `--bandeja` |
| **Desinstalar** | abre o desinstalador NSIS registrado, que chama o núcleo para restaurar PAC, tarefa e PATH antes de apagar os arquivos |

Fechar a janela a esconde na bandeja. Para encerrar apenas a interface, use
**Sair (o serviço continua)** no menu da bandeja.

## Os orbes de estado

O sinal de estado é o [`thinking-orbs`](https://github.com/Jakubantalik/thinking-orbs)
(MIT, canvas 2D, zero dependências), um orbe por situação, escolhido pelo que
o nome do orbe descreve:

| Situação | Orbe | Cor |
| --- | --- | --- |
| Conectando ao serviço | `connecting` — a constelação se ligando | destaque |
| Verificando | `solving` — as faixas embaralham e clicam resolvidas | destaque |
| Funcionando | `working` — partículas em órbita | verde |
| Pausado | `breathing` a meio ritmo — vivo, sem fazer nada | cinza |
| Procurando saída | `searching` — a meridiana varre o globo | âmbar |
| Parado | `shaping` — troca de forma sem assentar em nenhuma | vermelho |

**Nenhum orbe fica congelado.** A primeira versão usava `paused` em Pausado e
Parado, achando que orbe parado dizia "isto não está andando" — mas lê como
interface travada, não como serviço parado. Quem diz o que aconteceu é a
palavra e a frase ao lado; o orbe diz só o tom.

Os orbes são monocromáticos por design da biblioteca. A cor entra por um
filtro SVG em `Orbe.tsx`: uma gama no alfa (senão os pontos fracos somem no
off-white) e um `feFlood` com a cor da paleta. Isso preserva o desbotado de
cada ponto — um `hue-rotate` achataria tudo. `color-interpolation-filters:
sRGB` é obrigatório, senão a cor sai mais clara que a da paleta.

`prefers-reduced-motion` desenha um quadro estático, de graça: é a biblioteca
que trata.

## Decisões que fogem do desenho do handoff

- **Confirmar "Reiniciar Discord" acontece na própria linha**, não numa
  sobreposição — a janela só admite uma, e ela é a de desinstalar. A
  confirmação espera resposta e não some sozinha: o interruptor de autostart
  ocupa aquele mesmo lugar, e um clique atrasado desligaria o autostart.

- **O autostart é a tarefa, não a chave `Run`.** A interface compara o XML da
  tarefa com seu próprio caminho instalado e `--bandeja`; uma tarefa de mesmo
  nome com outra ação é conflito, nunca um falso “ligado”.

- **A marca é um "L" com a ponta virando** — o L de "Faz o L" e o desenho do
  que o programa faz. Mesmo traço na janela e na bandeja, onde ele muda de cor
  conforme o estado.

- **A lista de conexões mostra o que cada conexão é, não só o endereço.**
  `c-gru18-6fa2a6cb.discord.media` não diz nada a quem só quer transmitir a
  tela; "Servidor de voz — São Paulo" diz tudo, inclusive que a voz continua
  saindo pelo Brasil. O endereço cru segue ao lado, menor e selecionável, para
  quem vai colar numa issue. A tradução está em `util/hosts.ts`.

- **O cabeçalho da lista conta as rotas.** "0 pelo exterior · 11 direto" é a
  prova mais legível que existe de que Pausar funcionou — sem precisar ler
  host nenhum.

- **Inter vem embutida, só o latino e com eixo óptico.** São 73 KB de
  `inter-latin-opsz-normal.woff2`. A janela usa de 11,5 px a 22 px, e é o eixo
  `opsz` que faz esse intervalo caber numa fonte só: abre o espacejamento no
  miúdo e aperta no grande. As outras seis fatias (cirílico, grego, vietnamita)
  iam junto no `.exe` sem nunca serem desenhadas.

- **O texto secundário é `#57534E`, não o `#78716C` do handoff.** O tom
  original dá 4,2:1 sobre o off-white, abaixo do mínimo legível para corpo de
  texto. O novo dá 7:1 e continua claramente secundário. O tom antigo virou
  `texto3`, para o que é de fato incidental.

- **O "Iniciar com o PC" ganhou o peso visual de um botão** — borda, fundo e
  altura iguais aos dois ao lado. É a única coisa daquela linha que muda o
  comportamento do PC, e como rótulo cinza solto na direita ele sumia.

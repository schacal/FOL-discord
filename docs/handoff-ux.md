# Handoff — interface gráfica do FOL-discord

Este documento é autocontido. Quem for implementar não precisa ter acompanhado o desenvolvimento anterior.

---

## O que existe hoje

**Repositório:** `https://github.com/schacal/FOL-discord` (público, MIT)
**Linguagem:** Rust, binário único de ~1,8 MB, sem dependências de runtime
**Plataforma:** Windows apenas
**Estado:** funcionando e em uso. Release atual: v0.2.3

### O problema que o programa resolve

O Discord fixa a região da sessão a partir do IP que enxerga **no momento em que abre**. Em provedores brasileiros com peering ruim essa decisão sai errada, e o sintoma é a transmissão de tela e a câmera pararem de funcionar.

O programa faz o punhado de conexões que decidem essa região sair por um proxy estrangeiro. Voz, câmera e tela viajam em UDP e continuam saindo direto — o ping não muda, e o servidor de voz entregue continua sendo o brasileiro.

### Arquitetura atual

```
src/
├── main.rs        CLI (instalar/desinstalar/status/rodar) e laço principal
├── routing.rs     decide, por host, se a conexão sai pelo exterior ou direto
├── socks.rs       servidor SOCKS5 local em 127.0.0.1:9250
├── pool.rs        piscina de proxies públicos, com validação e auto-cura
├── pac.rs         servidor do arquivo PAC em 127.0.0.1:9251
├── discord.rs     localiza e reinicia o Discord
└── windows.rs     registro: AutoConfigURL, autostart e PATH
```

**Como se instala:** um processo em segundo plano sobe com o Windows pela chave `HKCU\...\Run\FolDiscord`. Ele serve um PAC em `127.0.0.1:9251`, e o `AutoConfigURL` do usuário aponta para ele. O PAC manda só o tráfego do Discord para o SOCKS local na 9250. O proxy local, por sua vez, manda `discord.com`, `gateway.discord.gg` e `latency.discord.media` para um proxy estrangeiro, e todo o resto direto.

**Estado em disco:**

| Caminho | Conteúdo |
| --- | --- |
| `%LOCALAPPDATA%\FolDiscord\fol-discord.exe` | o binário |
| `%LOCALAPPDATA%\FolDiscord\fol.log` | log, rotaciona em 512 KB, uma linha por conexão |
| `%LOCALAPPDATA%\FolDiscord\pronto` | marcador: existe quando a piscina tem proxies válidos |

**Chaves de registro, todas em HKCU:**

| Chave | Valor |
| --- | --- |
| `...\Internet Settings\AutoConfigURL` | `http://127.0.0.1:9251/proxy.pac` |
| `...\Internet Settings\AutoConfigURL_backup_FolDiscord` | valor anterior, para devolver na desinstalação |
| `...\CurrentVersion\Run\FolDiscord` | `"<caminho>\fol-discord.exe" rodar` |
| `Environment\Path` | acrescido de `%LOCALAPPDATA%\FolDiscord` |

---

## O que construir

Um aplicativo de bandeja com uma janela de gerenciamento. O usuário-alvo é leigo: ele quer ver se está funcionando, e desligar quando quebrar.

### Funcionalidades

1. **Ícone na bandeja**, sempre presente, com a cor indicando o estado (verde operacional, âmbar degradado, vermelho parado). Menu de clique direito com: abrir, pausar/retomar, sair.
2. **Status ao vivo**: operacional ou não, quantos proxies saudáveis, qual está em uso e de que país, e quando foi a última validação.
3. **Pausar e retomar** sem desinstalar. Pausado = o PAC responde `DIRECT` para tudo, e o Discord volta a sair pelo IP normal na próxima conexão.
4. **Ligar/desligar o início junto com o Windows** — um interruptor que escreve ou apaga a chave `Run`.
5. **Desinstalar** por um botão, com confirmação. Deve fazer exatamente o que `fol-discord desinstalar` faz hoje.
6. **Últimas conexões**: uma lista curta mostrando destino e se saiu pelo exterior ou direto. É o que dá confiança de que está funcionando, e é o que se pede para diagnosticar.

### Pilha recomendada

**Tauri v2 + React + shadcn/ui.**

O pedido original foi "Rust + React, mas o mais leve possível". A escolha está certa e vale explicar por quê, porque a alternativa parece mais leve e não é:

| Opção | Binário | Componentes prontos | Veredito |
| --- | --- | --- | --- |
| **Tauri v2** | ~4 MB | ecossistema web inteiro | **recomendado** |
| Electron | ~150 MB | mesmo ecossistema | pesado demais |
| egui / iced | ~6 MB | quase nada, e feio por padrão | não vale o esforço |

Tauri usa o **WebView2**, que já vem instalado no Windows 11 e na maioria dos Windows 10 atuais. Não empacota navegador nenhum — por isso 4 MB. E o backend é Rust, que é o que o projeto já é. Não há ganho real em ir de pura-Rust: economiza 2 MB e custa toda a biblioteca de componentes.

Use `shadcn/ui` para os componentes e Tailwind para o resto. Tauri v2 tem suporte nativo a bandeja (`tauri-plugin-tray`) e a iniciar com o sistema.

---

## O trabalho de backend que precisa existir antes

**Hoje não há como a interface conversar com o serviço.** Ela não deve reimplementar nada nem ficar analisando o log — precisa de um canal de controle. Este é o primeiro passo, e é dentro do repositório atual.

### Adicionar: API de controle local

Um servidor HTTP em `127.0.0.1:9252`, escutando **só** em loopback, dentro do processo que já roda. Sem autenticação — a porta não é alcançável de fora, e o mesmo raciocínio já vale para as portas 9250 e 9251.

```
GET  /status      -> 200 StatusJson
POST /pausar      -> 204
POST /retomar     -> 204
GET  /conexoes    -> 200 { conexoes: Conexao[] }   últimas 50, mais recentes primeiro
POST /revalidar   -> 204   força um reabastecimento imediato da piscina
```

```jsonc
// StatusJson
{
  "versao": "0.2.3",
  "operacional": true,        // pac ligado && serviço de pé && proxies > 0
  "pausado": false,
  "autostart": true,
  "pac_ligado": true,
  "proxies_saudaveis": 15,
  "proxy_em_uso": { "endereco": "95.81.103.220:1080", "regiao": "rotterdam", "latencia_ms": 1031 },
  "ultima_validacao_utc": "2026-09-01T04:12:33Z"
}

// Conexao
{ "hora_utc": "2026-09-01T04:15:02Z", "host": "discord.com", "porta": 443, "rota": "exterior" }
```

### Notas de implementação

- **Pausar** deve virar um estado em memória que o `pac.rs` consulta: pausado faz o PAC responder `DIRECT` para tudo. Não mexa no registro para pausar — isso deixa lixo se o programa morrer no meio.
- **Últimas conexões** devem virar um anel em memória de 50 posições preenchido em `socks.rs`, onde hoje já se chama `log::linha`. Não analise o arquivo de log: ele rotaciona e o formato não é um contrato.
- **Autostart e desinstalar** já existem em `windows.rs` e `main.rs`. Reaproveite, não reescreva.
- Ao adicionar a API, mantenha `fol-discord status` funcionando: quem usa terminal não deve ser obrigado a abrir a interface.

---

## Restrições que não podem ser quebradas

1. **Sem administrador.** Tudo em `HKCU` e `%LOCALAPPDATA%`. Se a interface precisar de elevação para algo, o desenho está errado.
2. **A interface é opcional.** O serviço tem que continuar funcionando sozinho com a interface fechada, e continuar subindo com o Windows sem ela. A interface gerencia; ela não é o programa.
3. **Nada de certificado raiz, nada de tocar em arquivo do Discord.** É o que sustenta a página de segurança do projeto. Ver `docs/seguranca.md`.
4. **Desinstalar tem que devolver o `AutoConfigURL` anterior**, que está guardado em `AutoConfigURL_backup_FolDiscord`. Não apague a chave às cegas.
5. **Português do Brasil** em toda a interface, como no resto do projeto.
6. **Tudo compila no GitHub Actions.** Veja `.github/workflows/release.yml`. Nada de binário enviado da máquina de alguém.

---

## Como saber que ficou pronto

- [ ] Instalação continua sendo a mesma linha única de PowerShell, e passa a instalar a interface junto.
- [ ] Ícone na bandeja aparece após o boot, sem janela piscando.
- [ ] Pausar faz a próxima conexão do Discord sair direto; retomar faz voltar pelo exterior. Verificável em `%LOCALAPPDATA%\FolDiscord\fol.log`.
- [ ] O interruptor de autostart de fato cria e apaga a chave `Run`.
- [ ] Desinstalar pela interface deixa o sistema igual a `fol-discord desinstalar`: registro devolvido, PATH limpo, pasta apagada.
- [ ] Derrubar todos os proxies (bloqueie as portas de saída) faz o status virar não-operacional em até 5 minutos, **e o Discord continua funcionando** — só sem a correção.
- [ ] O binário total continua abaixo de 10 MB.

---

## Problema conhecido, e é uma boa primeira tarefa

`fol-discord status` **com a saída redirecionada** (`> arquivo.txt` ou por pipe) sai vazio. Numa janela interativa parece funcionar; redirecionado, não.

A causa está em `anexar_console()` no `main.rs`. O binário é compilado com `#![windows_subsystem = "windows"]` para não piscar console no autostart, e por isso precisa adotar o console de quem o chamou e reabrir as saídas padrão em `CONOUT$`. A lógica que decide entre "usar a saída herdada" e "adotar o console" não está acertando os dois casos.

Vale resolver de uma vez, porque a interface vai depender de invocar o binário. **A saída mais limpa provavelmente é separar em dois executáveis**: `fol-discord.exe` como aplicativo de console de verdade, para a linha de comando, e `fol-discord-service.exe` sem console, para o autostart. Isso elimina a classe inteira de problema em vez de continuar remendando.

# Desenvolvimento

Como o repositório está organizado, e como compilar e testar cada metade.

## Estrutura

```
fol-discord/
├── src/                    o serviço, em Rust
│   ├── main.rs        instalação, desinstalação, status, laço principal
│   ├── routing.rs     decide, por host, quem sai por fora
│   ├── socks.rs       o proxy local em 127.0.0.1:9250
│   ├── pool.rs        piscina de proxies públicos, com auto-cura
│   ├── pac.rs         o arquivo PAC que o Windows lê
│   ├── discord.rs     encontra e reinicia o Discord
│   └── windows.rs     registro: PAC, autostart, PATH — e como desfazê-los
├── interface/              a janela instaladora, em Tauri + React
│   ├── src/           a janela: estados, métricas, atividade
│   ├── src-tauri/     a moldura, a bandeja, os ícones e o setup NSIS
│   └── scripts/       ícones da bandeja e os testes de embalagem
├── docs/
├── assets/                 materiais visuais do repositório
│   ├── banner.png         capa do README
│   ├── demo.gif           demonstração breve da janela
│   └── icons/             logo principal e ícone de bandeja
├── install.ps1
└── .github/workflows/release.yml
```

São dois projetos Rust separados. O da raiz é o serviço; o de
`interface/src-tauri/` é a janela, que **embute uma cópia compilada do serviço**
no próprio executável — é assim que a primeira abertura instala tudo sozinha.

## O serviço

```bash
cargo build --release   # sai em target/release/fol-discord.exe
cargo test
```

## A janela

```bash
npm --prefix interface install
npm --prefix interface run dev
```

Isso abre a janela no navegador, no tamanho real, falando com um serviço
simulado — dá para ver e testar os quatro estados sem Rust e sem proxy nenhum.

Para gerar o instalador de verdade:

```bash
npm --prefix interface run tauri build --bundles nsis
npm --prefix interface test
```

O segundo comando é a suíte de embalagem: ela confere o artefato compilado, não
só o código. Verifica que o serviço realmente foi embutido no executável da
janela, que nenhum processo auxiliar abre terminal preto, que o ícone do
programa é a logo ilustrada, que a chave de desinstalação bate com o
`productName` do `tauri.conf.json`, e que a embalagem produziu um único
`*-setup.exe`. Rode-a **depois** do build.

## Ícones

```bash
npx tauri icon ../assets/icons/app.png -o src-tauri/icones   # a partir de interface/
node scripts/icones.mjs
```

O primeiro gera o ícone do programa a partir da mesma logo ilustrada que o
cabeçalho da janela mostra. O segundo desenha as quatro cores da bandeja, que
continuam sendo o "L" por fórmula porque precisam mudar de cor por estado e
continuar legíveis em 16 px. Só rode se mexer na marca; os arquivos já estão no
repositório.

## Versões

Cinco lugares precisam concordar a cada release:

| Arquivo | Campo |
| --- | --- |
| `Cargo.toml` | `version` |
| `interface/src-tauri/Cargo.toml` | `version` |
| `interface/package.json` | `version` |
| `interface/src-tauri/tauri.conf.json` | `version` |
| `interface/src/App.tsx` | `VERSAO_PADRAO` |

O último é o que a janela mostra antes de o serviço responder.

## Release

Empurrar uma tag `v*` dispara o `.github/workflows/release.yml`, que compila no
GitHub Actions e publica **um arquivo só**: o `FOL-discord-setup.exe`. O
instalador publicado nunca é enviado da máquina de ninguém.

```bash
git tag -a v0.2.5 -m "v0.2.5"
git push origin v0.2.5
```

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
├── packaging/              receitas e verificações dos pacotes Linux
├── docs/
├── assets/                 materiais visuais do repositório
│   ├── banner.png         capa do README
│   ├── demo.gif           demonstração breve da janela
│   └── icons/             logo principal e ícone de bandeja
├── install.ps1
└── .github/workflows/release.yml
```

São dois projetos Rust separados. O da raiz é o serviço; o de
`interface/src-tauri/` é a janela. O `build.rs` da janela compila o serviço e o
entrega ao empacotador como **sidecar**: o NSIS grava `fol-discord.exe` ao lado
de `fol-discord-janela.exe`; os pacotes Linux gravam `fol-discord` ao lado de
`fol-discord-janela`. Na primeira abertura, a janela instala uma cópia por
usuário e a inicia.

Até a v0.2.5 a janela carregava o serviço inteiro como dado (`include_bytes!`)
e o gravava em disco ao abrir. Um executável completo dentro da seção de dados
de outro, extraído em tempo de execução, é o desenho que os antivírus chamam de
*dropper* — e era desnecessário, porque o instalador já entrega o arquivo. A
cópia embutida sobrou só em `cargo tauri dev`, onde não existe instalador para
colocar o serviço no lugar.

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
só o código. Verifica que o serviço saiu como sidecar e **não** voltou a ser
embutido na janela, que nem o serviço nem a janela chamam `tasklist` ou
`taskkill`, que nenhum processo auxiliar abre terminal preto, que o ícone do
programa é a logo ilustrada, que a chave de desinstalação bate com o
`productName` do `tauri.conf.json`, que a release publica os dois nomes de
instalador, e que a embalagem produziu um único `*-setup.exe`. Rode-a
**depois** do build.

### Dependências e pacotes Linux

Use os pacotes de desenvolvimento equivalentes da sua distribuição:

| Família | Dependências principais |
| --- | --- |
| Debian / Ubuntu | `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf` |
| Fedora / RHEL | `webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel patchelf` |
| Arch Linux | `webkit2gtk-4.1 libayatana-appindicator librsvg patchelf` |
| openSUSE | `webkit2gtk3-devel libappindicator3-1 librsvg-devel patchelf` |

Depois, gere e confira os três formatos do Tauri:

```bash
npm ci --prefix interface
npm --prefix interface run tauri -- build --bundles deb,rpm,appimage
bash packaging/linux/verificar-pacotes.sh
```

O `.deb` atende Debian/Ubuntu, o `.rpm` atende Fedora/RHEL e o AppImage é a
saída portátil para openSUSE. Arch usa `packaging/arch/PKGBUILD`, que a release
compila em um contêiner Arch e publica como `.pkg.tar.zst`.

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

Sete lugares precisam concordar a cada release:

| Arquivo | Campo |
| --- | --- |
| `Cargo.toml` | `version` |
| `interface/src-tauri/Cargo.toml` | `version` |
| `interface/package.json` | `version` |
| `interface/src-tauri/tauri.conf.json` | `version` |
| `interface/src/App.tsx` | `VERSAO_PADRAO` |
| `packaging/arch/PKGBUILD` | `pkgver` |
| `packaging/arch/.SRCINFO` | `pkgver` e URL do tag |

O último é o que a janela mostra antes de o serviço responder. O
`interface/package-lock.json` acompanha o `package.json`.

A tag **precisa** bater com esses cinco: o NSIS carimba a versão do
`tauri.conf.json` no nome do instalador, e o workflow recusa publicar se a tag
pedir outro nome. Sem essa trava, quem já instalou nunca receberia o aviso de
versão nova.

## Release

Empurrar uma tag `v*` dispara o `.github/workflows/release.yml`, que compila no
GitHub Actions para Windows e Linux e publica a release. Nenhum instalador ou
pacote publicado vem da máquina de alguém.

```bash
git tag -a v0.2.7 -m "v0.2.7"
git push origin v0.2.7
```

Cada release publica:

| Arquivo | Para quê |
| --- | --- |
| `FOL-discord-setup.exe` | nome fixo; é o que o botão do README e o `install.ps1` baixam |
| `FOL-discord_<versão>_x64-setup.exe` | o mesmo arquivo, com o nome que o NSIS carimba; é o que a janela instalada procura para avisar de versão nova |
| `SHA256SUMS.txt` | a soma do instalador, para quem quer conferir na mão |
| atestado de procedência | assinado pelo GitHub; amarra o arquivo ao commit e à execução que o produziu |
| `FOL-discord-x86_64.deb` | Debian e Ubuntu |
| `FOL-discord-x86_64.rpm` | Fedora e distribuições compatíveis com RPM |
| `FOL-discord-x86_64.AppImage` | execução portátil, inclusive no openSUSE |
| `fol-discord-<versão>-1-x86_64.pkg.tar.zst` | Arch Linux |

Antes de empurrar a tag, anote a versão em `CHANGELOG.md` e rode as três
suítes (`cargo test --release` na raiz, `cargo test` em `interface/src-tauri` e
`npm --prefix interface test` depois do build).

### Como a janela descobre uma versão nova

A janela consulta a última release pública deste repositório ao abrir, cada
vez que é trazida da bandeja para a frente (com folga mínima de dez minutos
entre consultas, para abrir-e-fechar não virar rajada de requisições) e, de
resto, a cada seis horas. Ela só avisa se a release for estável (nem
*draft*, nem *prerelease*), mais nova que a versão instalada, e trouxer o
instalador **dentro da própria release** — primeiro pelo nome com versão, depois
pelo nome fixo. O clique no aviso abre o download no navegador; a janela nunca
baixa nem executa nada sozinha. A lógica e os testes estão em
`interface/src-tauri/src/servico.rs`.

### Assinatura do instalador

O instalador **sai sem assinatura de código**. Não é escolha: certificado de
confiança pública custa dinheiro, e as duas rotas mais conhecidas não estão
abertas para este projeto hoje:

- **Certificado tradicional (OV/EV)** — pago, anual, e ainda assim o
  SmartScreen só para de avisar depois de o certificado acumular reputação.
- **Microsoft Artifact Signing (ex-Trusted Signing)** — mensal e mais barato,
  mas a validação de identidade de pessoa física só é aceita para quem mora
  nos Estados Unidos ou no Canadá. Pessoa jurídica precisa estar num dos
  países listados na [documentação](https://learn.microsoft.com/en-us/azure/artifact-signing/quickstart),
  e o Brasil não está.

A rota gratuita que existe para software livre é a
[**SignPath Foundation**](https://signpath.org/): ela empresta certificado de
código a projetos de código aberto que atendam aos critérios dela — licença
aprovada pela OSI, projeto mantido e já lançado, build automatizado e
verificável a partir do próprio repositório, autenticação em dois fatores para
quem assina. Este repositório já cumpre a parte técnica; falta a inscrição, que
é manual e passa por revisão humana.

Enquanto não há certificado, a release publica o que dá para provar sem ele: a
soma SHA-256 e o atestado de procedência do GitHub. É o que o `install.ps1` e a
[página de segurança](seguranca.md#conferindo-o-instalador-que-você-baixou)
usam.

O workflow já sabe assinar quando houver como. Ele procura seis valores no
repositório do GitHub:

- **Secrets:** `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET` e `AZURE_TENANT_ID`.
- **Variables:** `AZURE_ARTIFACT_SIGNING_ENDPOINT`,
  `AZURE_ARTIFACT_SIGNING_ACCOUNT` e `AZURE_ARTIFACT_SIGNING_PROFILE`.

Com os seis, a tag instala o `artifact-signing-cli`, assina o núcleo antes de
empacotá-lo, entrega ao `signCommand` do Tauri a assinatura da janela e do setup
NSIS, e recusa publicar se qualquer assinatura sair inválida. Sem eles, a
release **sai mesmo assim**, sem assinatura, avisando no log do workflow — um
projeto sem nenhum jeito de publicar correção é pior do que um binário sem
assinatura que admite não ter. Pull requests e builds da branch `main` nunca
recebem as credenciais.

Se um dia a assinatura vier pela SignPath em vez do Azure, a etapa a trocar é a
mesma: o `signCommand` do Tauri e o passo que assina o núcleo.

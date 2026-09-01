import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { coresDistintas, lerPng } from "./png.mjs";

const daqui = fileURLToPath(new URL(".", import.meta.url));
const janela = new URL(
  "../src-tauri/target/release/fol-discord-janela.exe",
  import.meta.url,
);
const ponteNativa = new URL("../src-tauri/src/servico.rs", import.meta.url);
const inicializacao = new URL("../src-tauri/src/inicializacao.rs", import.meta.url);
const cicloDeVida = new URL("../src-tauri/src/main.rs", import.meta.url);
const configuracaoTauri = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const hooksNsis = new URL("../src-tauri/windows/hooks.nsh", import.meta.url);
const nucleo = new URL("../../src/main.rs", import.meta.url);
const controleDiscord = new URL("../../src/discord.rs", import.meta.url);
const tela = new URL("../src/App.tsx", import.meta.url);
const marca = new URL("../src/componentes/Marca.tsx", import.meta.url);
const logoPrincipal = new URL("../../assets/icons/app.png", import.meta.url);
const pastaDeBuild = fileURLToPath(
  new URL("../src-tauri/target/release/build/", import.meta.url),
);
const pastaNsis = fileURLToPath(
  new URL("../src-tauri/target/release/bundle/nsis/", import.meta.url),
);

async function servicoQueFoiEmbutido() {
  const pastas = await readdir(pastaDeBuild, { withFileTypes: true });
  const candidatos = pastas
    .filter((entrada) => entrada.isDirectory() && entrada.name.startsWith("fol-discord-janela-"))
    .map((entrada) => join(pastaDeBuild, entrada.name, "out", "fol-discord.exe"));

  for (const candidato of candidatos) {
    try {
      return await readFile(candidato);
    } catch {
      // Há uma pasta por tentativa de compilação; só uma contém o artefato.
    }
  }
  throw new Error("serviço embutido ausente da compilação da janela");
}

test("o executável da janela carrega o serviço que instala", async () => {
  // A leitura é binária: procurar o executável inteiro impede publicar uma
  // janela que apenas conhece o nome do serviço, mas não consegue instalá-lo.
  const [servicoEmbutido, janelaCompilada] = await Promise.all([
    servicoQueFoiEmbutido(),
    readFile(janela),
  ]);

  assert.ok(servicoEmbutido.length > 1_000_000, "serviço compilado ausente");
  assert.ok(janelaCompilada.includes(servicoEmbutido), "a janela foi compilada sem o serviço");
});

test("as sondas da janela não podem abrir um terminal visível", async () => {
  const ponte = await readFile(ponteNativa, "utf8");
  assert.match(
    ponte,
    /fn servico_rodando\(\) -> bool \{\s*let mut comando = comando_oculto\("tasklist"\);/s,
    "a leitura periódica do estado iniciaria tasklist com uma janela visível",
  );
});

test("o instalador e o desinstalador não podem abrir terminal visível", async () => {
  const [servico, discord] = await Promise.all([
    readFile(nucleo, "utf8"),
    readFile(controleDiscord, "utf8"),
  ]);

  for (const fonte of [servico, discord]) {
    assert.match(
      fonte,
      /fn comando_oculto\(programa: impl AsRef<OsStr>\).*CREATE_NO_WINDOW/s,
      "um comando auxiliar do núcleo ainda pode abrir uma janela",
    );
    assert.doesNotMatch(
      fonte,
      /std::process::Command::new\(/,
      "um comando do núcleo ignorou a execução oculta",
    );
  }
});

test("a janela usa a logo principal e a bandeja continua dinâmica", async () => {
  const [marcaFonte, logo, bandeja] = await Promise.all([
    readFile(marca, "utf8"),
    readFile(logoPrincipal),
    readFile(cicloDeVida, "utf8"),
  ]);

  assert.match(marcaFonte, /app\.png/, "a janela ainda usa a marca SVG antiga");
  assert.ok(
    logo.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])),
    "a logo principal precisa ser um PNG válido",
  );
  assert.match(bandeja, /bandeja-operacional\.png/);
  assert.match(bandeja, /bandeja-pausado\.png/);
  assert.match(bandeja, /bandeja-sem_proxies\.png/);
  assert.match(bandeja, /bandeja-parado\.png/);
});

test("o ícone do programa é a mesma logo ilustrada do cabeçalho", async () => {
  // O que separa as duas marcas é o número de cores: a logo ilustrada tem
  // milhares, a marca desenhada por fórmula tem o indigo chapado, o branco do
  // "L" e o antisserrilhado entre eles. Enquanto o `.ico` saía da fórmula, a
  // janela mostrava uma logo na tela e outra na barra de tarefas.
  const [logo, janelaIcone, pequeno, bandejaIcone] = await Promise.all([
    readFile(logoPrincipal),
    readFile(new URL("../src-tauri/icones/icon.png", import.meta.url)),
    readFile(new URL("../src-tauri/icones/128x128.png", import.meta.url)),
    readFile(new URL("../src-tauri/icones/bandeja-operacional.png", import.meta.url)),
  ]);

  const cores = coresDistintas(logo);
  assert.ok(cores > 1000, `a logo principal deveria ser ilustrada, tem ${cores} cores`);
  assert.ok(
    coresDistintas(janelaIcone) > 1000,
    "icones/icon.png voltou a ser a marca desenhada por fórmula",
  );
  assert.ok(
    coresDistintas(pequeno) > 500,
    "icones/128x128.png voltou a ser a marca desenhada por fórmula",
  );
  assert.equal(lerPng(janelaIcone).largura, lerPng(logo).largura);

  // A bandeja é o contrário de propósito: ela precisa continuar chapada, para
  // mudar de cor por estado e continuar legível em 16 px.
  assert.ok(
    coresDistintas(bandejaIcone) < 500,
    "a bandeja precisa continuar sendo a marca por fórmula, uma cor por estado",
  );
});

test("o botão Desinstalar procura a chave que o setup NSIS realmente grava", async () => {
  // O template do Tauri monta a chave de desinstalação com o `productName`,
  // não com o `identifier`. Procurar pelo identificador achava sempre nada, e
  // o botão só falhava depois de instalado pelo setup — nunca em `dev`.
  const [fonte, configuracaoTexto] = await Promise.all([
    readFile(inicializacao, "utf8"),
    readFile(configuracaoTauri, "utf8"),
  ]);
  const { productName } = JSON.parse(configuracaoTexto);

  // Montada por partes: uma chave do registro cheia de contrabarras dentro de
  // uma expressão regular vira um enigma de escapes, e o enigma esconde o erro.
  const chaveEsperada = [
    "Software",
    "Microsoft",
    "Windows",
    "CurrentVersion",
    "Uninstall",
    productName,
  ].join("\\");

  assert.ok(
    fonte.includes(`r"${chaveEsperada}"`),
    `a chave de desinstalação precisa terminar em ${productName}`,
  );
  assert.match(
    fonte,
    /NOME_DESINSTALADOR: &str = "uninstall\.exe"/,
    "sem o desinstalador vizinho, apagar o registro deixa a janela sem botão",
  );
});

test("desinstalar pelo setup remove a tarefa de logon que a janela criou", async () => {
  // A tarefa é criada pela janela, então o desinstalador do NSIS não a conhece
  // sozinho. Deixá-la para trás faz o Windows tentar abrir um .exe apagado a
  // cada login.
  const hooks = await readFile(hooksNsis, "utf8");

  assert.match(hooks, /schtasks\.exe" \/delete \/tn "FolDiscord\.Bandeja" \/f/);
  const execs = hooks.match(/nsExec::ExecToLog/g) ?? [];
  const pops = hooks.match(/^\s*Pop \$0$/gm) ?? [];
  assert.equal(
    execs.length,
    pops.length,
    "cada nsExec deixa um código na pilha; sem o Pop, o StrCmp lê o comando errado",
  );
});

test("a última checagem é carimbada pelo serviço, não só pelo botão", async () => {
  // Enquanto só `verificar` escrevia, a coluna ficava em travessão para sempre
  // para quem nunca clicava em "Verificar agora".
  const [servico, ponte] = await Promise.all([
    readFile(nucleo, "utf8"),
    readFile(ponteNativa, "utf8"),
  ]);

  const arquivo = /join\("ultima-validacao-ms"\)/;
  assert.match(servico, arquivo, "o serviço não conhece o arquivo da checagem");
  assert.match(ponte, arquivo, "a janela lê outro arquivo");
  assert.match(
    servico,
    /registrar_checagem_em\(&caminho_ultima_validacao\(\), milissegundos_agora\(\)\)/,
    "o laço de manutenção não carimba a passada que acabou de fazer",
  );
  assert.match(
    servico,
    /let _ = std::fs::remove_file\(caminho_ultima_validacao\(\)\)/,
    "uma instalação nova herdaria a checagem da instalação anterior",
  );
});

test("reiniciar Discord não pode aguardar a validação de proxies", async () => {
  const [ponte, servico] = await Promise.all([
    readFile(ponteNativa, "utf8"),
    readFile(nucleo, "utf8"),
  ]);

  assert.match(
    ponte,
    /pub fn reiniciar_discord\(\).*comando_oculto\(&executavel\).*arg\("reiniciar-discord"\)/s,
    "o botão ainda executaria a instalação completa",
  );
  assert.match(
    servico,
    /"reiniciar-discord"\s*=>\s*reiniciar_discord\(\)/,
    "o núcleo não expõe o reinício direto",
  );
});

test("a ajuda do núcleo documenta o reinício direto do Discord", async () => {
  const servico = await readFile(nucleo, "utf8");
  assert.match(
    servico,
    /fol-discord reiniciar-discord\s+fecha e abre só o Discord/,
    "o comando exposto no README não aparece na ajuda do programa",
  );
});

test("verificar agora inicia o serviço apenas uma vez", async () => {
  const app = await readFile(tela, "utf8");
  const inicio = app.indexOf("const verificar =");
  const fim = app.indexOf("const reiniciarDiscord =", inicio);
  const acao = app.slice(inicio, fim);

  assert.ok(inicio >= 0 && fim > inicio, "não encontrei a ação de verificação");
  assert.doesNotMatch(
    acao,
    /religarServico/,
    "a tela pediria duas inicializações concorrentes",
  );
});

test("a tarefa da bandeja usa somente o contrato seguro de logon", async () => {
  const fonte = await readFile(inicializacao, "utf8");

  assert.match(fonte, /TAREFA_BANDEJA: &str = "FolDiscord\.Bandeja"/);
  // No XML do Agendador, LogonTrigger e LeastPrivilege são os equivalentes
  // verificáveis de ONLOGON e LIMITED, sem depender do idioma do Windows.
  assert.match(fonte, /<LogonTrigger>/);
  assert.match(fonte, /<RunLevel>LeastPrivilege<\/RunLevel>/);
  assert.match(fonte, /--bandeja/);
  assert.match(fonte, /\.fol-discord-instalada/);
  assert.doesNotMatch(fonte, /\/RL\s+HIGHEST/i);
});

test("o boot cria a bandeja sem mostrar a janela nem recriar o Run legado", async () => {
  const [main, ponte] = await Promise.all([
    readFile(cicloDeVida, "utf8"),
    readFile(ponteNativa, "utf8"),
  ]);

  assert.match(
    main,
    /let boot = std::env::args\(\)\.any\(\|arg\| arg == SEM_JANELA\)/,
    "--bandeja precisa ser decidido uma única vez antes do ciclo da janela",
  );
  assert.match(
    main,
    /if !boot \{[\s\S]*janela\.show\(\)/,
    "--bandeja não pode mostrar a janela",
  );
  assert.match(main, /icone\("inicializando"\)/, "a bandeja deve comunicar preparação");
  assert.match(
    ponte,
    /arg\("--sem-autostart"\)/,
    "a interface não pode recriar Run legado",
  );
  assert.match(
    ponte,
    /comando_desinstalador\(\)/,
    "a interface instalada deve usar o desinstalador NSIS",
  );
});

test("o setup NSIS permanece por usuário e preserva a limpeza do núcleo", async () => {
  const [configuracaoTexto, hooks] = await Promise.all([
    readFile(configuracaoTauri, "utf8"),
    readFile(hooksNsis, "utf8"),
  ]);
  const configuracao = JSON.parse(configuracaoTexto);

  assert.equal(configuracao.bundle.active, true);
  assert.deepEqual(configuracao.bundle.targets, ["nsis"]);
  assert.equal(configuracao.bundle.windows.nsis.installMode, "currentUser");
  assert.equal(configuracao.bundle.windows.webviewInstallMode.type, "downloadBootstrapper");
  assert.match(hooks, /NSIS_HOOK_POSTINSTALL/);
  assert.match(hooks, /NSIS_HOOK_PREUNINSTALL/);
  assert.match(hooks, /desinstalar --manter-arquivos/);
});

test("a embalagem produz exatamente um setup para download", async () => {
  const arquivos = await readdir(pastaNsis, { withFileTypes: true });
  const setups = arquivos.filter(
    (arquivo) => arquivo.isFile() && /-setup\.exe$/i.test(arquivo.name),
  );

  assert.equal(setups.length, 1, "a embalagem deve conter um único *-setup.exe");
});

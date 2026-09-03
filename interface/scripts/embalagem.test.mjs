import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

import { coresDistintas, lerPng } from "./png.mjs";

const daqui = fileURLToPath(new URL(".", import.meta.url));
const windows = process.platform === "win32";
const sufixoExecutavel = windows ? ".exe" : "";
const alvo =
  process.env.TARGET ??
  `${process.arch === "arm64" ? "aarch64" : "x86_64"}-${
    windows ? "pc-windows-msvc" : "unknown-linux-gnu"
  }`;
const janela = new URL(
  `../src-tauri/target/release/fol-discord-janela${sufixoExecutavel}`,
  import.meta.url,
);
const ponteNativa = new URL("../src-tauri/src/servico.rs", import.meta.url);
const inicializacao = new URL("../src-tauri/src/inicializacao.rs", import.meta.url);
const cicloDeVida = new URL("../src-tauri/src/main.rs", import.meta.url);
const configuracaoTauri = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const configuracaoLinux = new URL(
  "../src-tauri/tauri.linux.conf.json",
  import.meta.url,
);
const hooksNsis = new URL("../src-tauri/windows/hooks.nsh", import.meta.url);
const releaseWorkflow = new URL("../../.github/workflows/release.yml", import.meta.url);
const nucleo = new URL("../../src/main.rs", import.meta.url);
const controleDiscord = new URL("../../src/discord.rs", import.meta.url);
const tela = new URL("../src/App.tsx", import.meta.url);
const marca = new URL("../src/componentes/Marca.tsx", import.meta.url);
const logoPrincipal = new URL("../../assets/icons/app.png", import.meta.url);
const pastaSidecar = fileURLToPath(new URL("../src-tauri/binaries/", import.meta.url));
const pastaNsis = fileURLToPath(
  new URL("../src-tauri/target/release/bundle/nsis/", import.meta.url),
);
const receitaArch = new URL("../../packaging/arch/PKGBUILD", import.meta.url);

async function nucleoCompilado() {
  const candidatos = [
    new URL(
      `../../target/${alvo}/release/fol-discord${sufixoExecutavel}`,
      import.meta.url,
    ),
    new URL(`../../target/release/fol-discord${sufixoExecutavel}`, import.meta.url),
  ];
  for (const candidato of candidatos) {
    try {
      return await readFile(candidato);
    } catch (erro) {
      if (erro?.code !== "ENOENT") throw erro;
    }
  }
  throw new Error(`o núcleo não foi compilado para ${alvo}`);
}

async function sidecarDoServico() {
  const arquivos = await readdir(pastaSidecar);
  const esperado = `fol-discord-${alvo}${sufixoExecutavel}`;
  const nome = arquivos.find((arquivo) => arquivo === esperado);
  if (!nome) throw new Error("o sidecar do serviço não foi gerado pelo build.rs");
  return readFile(join(pastaSidecar, nome));
}

test("o instalador leva o serviço como sidecar, e a janela não o carrega como dado", async () => {
  // A leitura é binária dos dois lados. O sidecar precisa ser exatamente o
  // núcleo compilado — publicar uma janela que só conhece o nome do serviço
  // continua sendo o defeito que este teste existe para impedir.
  const [sidecar, nucleoBinario, janelaCompilada] = await Promise.all([
    sidecarDoServico(),
    nucleoCompilado(),
    readFile(janela),
  ]);

  assert.ok(sidecar.length > 1_000_000, "serviço compilado ausente");
  const assinatura = windows ? Buffer.from("MZ") : Buffer.from([0x7f, 0x45, 0x4c, 0x46]);
  assert.ok(
    sidecar.subarray(0, assinatura.length).equals(assinatura),
    "o sidecar não é um executável da plataforma",
  );

  // Tamanho aproximado, não igualdade byte a byte: na release assinada o
  // empacotador pode carimbar a assinatura no sidecar depois da cópia, e aí os
  // bytes divergem legitimamente. O que precisa ser impossível é o sidecar ser
  // outro arquivo qualquer.
  const diferenca = Math.abs(sidecar.length - nucleoBinario.length);
  assert.ok(
    diferenca < nucleoBinario.length * 0.05,
    `o sidecar (${sidecar.length} B) não parece ser o núcleo (${nucleoBinario.length} B)`,
  );

  // Um PE completo dentro da seção de dados de outro PE é o padrão que os
  // motores de antivírus leem como conta-gotas. O sidecar existe justamente
  // para não precisarmos disso: o instalador entrega o arquivo.
  assert.ok(
    !janelaCompilada.includes(sidecar),
    "a janela voltou a carregar o serviço como blob embutido",
  );
});

test("as sondas da janela não iniciam processo nenhum", async () => {
  // Antes a leitura periódica do estado chamava `tasklist` com a janela
  // suprimida. Ela deixou de chamar processo algum: enumerar pela API não
  // pisca console, não depende do idioma do Windows e tira da árvore de
  // processos duas chamadas que os antivírus contam como comportamento de
  // programa malicioso.
  const ponte = await readFile(ponteNativa, "utf8");
  assert.match(
    ponte,
    /fn servico_rodando\(\) -> bool \{\s*crate::processos::esta_rodando\(plataforma::NOME_SERVICO\)\s*\}/s,
    "a sonda de estado voltou a depender de um utilitário externo",
  );
  for (const utilitario of ["tasklist", "taskkill"]) {
    assert.ok(
      !ponte.includes(`"${utilitario}"`),
      `a janela voltou a chamar ${utilitario}`,
    );
  }
});

test("nem o núcleo nem a janela chamam tasklist ou taskkill", async () => {
  // Enumerar processos e encerrar programa de terceiros por utilitário do
  // sistema são duas das detecções comportamentais de maior peso em sandbox.
  // O módulo `processos` faz as duas coisas pela API, sem processo filho.
  const fontes = await Promise.all(
    [nucleo, controleDiscord, ponteNativa].map((arquivo) => readFile(arquivo, "utf8")),
  );
  for (const fonte of fontes) {
    for (const utilitario of ["tasklist", "taskkill"]) {
      assert.ok(!fonte.includes(`"${utilitario}"`), `${utilitario} voltou ao código`);
    }
  }
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
  // A tarefa é criada pela janela, então a limpeza continua pertencendo ao
  // serviço, que já conhece o backup do PAC e valida o autostart do FOL antes
  // de remover qualquer coisa.
  const hooks = await readFile(hooksNsis, "utf8");

  assert.match(
    hooks,
    /!insertmacro CheckIfAppIsRunning "fol-discord-janela\.exe" "FOL-discord"/,
    "o hook deve bloquear a limpeza se a janela ainda estiver aberta",
  );
  assert.doesNotMatch(
    hooks,
    /schtasks\.exe/i,
    "a remoção da tarefa deve ter um único dono no serviço",
  );
  const execs = hooks.match(/nsExec::ExecToLog/g) ?? [];
  const pops = hooks.match(/^\s*Pop \$0$/gm) ?? [];
  assert.equal(
    execs.length,
    pops.length,
    "cada nsExec deixa um código na pilha; sem o Pop, o StrCmp lê o comando errado",
  );
});

test("o hook NSIS delega a limpeza ao serviço sem matar processos por utilitário", async () => {
  const hooks = await readFile(hooksNsis, "utf8");

  assert.match(
    hooks,
    /fol-discord\.exe" desinstalar --manter-arquivos/,
    "a limpeza deve continuar sendo feita pelo serviço que conhece o backup do usuário",
  );
  assert.doesNotMatch(
    hooks,
    /taskkill\.exe/i,
    "o setup não deve carregar um encerramento forçado via taskkill",
  );
  assert.doesNotMatch(
    hooks,
    /schtasks\.exe/i,
    "a remoção da tarefa deve ter um único dono no serviço",
  );
  assert.equal(
    (hooks.match(/nsExec::ExecToLog/g) ?? []).length,
    1,
    "o hook deve executar somente a limpeza do serviço",
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
  assert.match(fonte, /join\("fol-discord"\)\.is_file\(\)/);
  assert.match(fonte, /env::var_os\("APPIMAGE"\)/);
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

test("a embalagem produz exatamente um setup para download", { skip: !windows }, async () => {
  const arquivos = await readdir(pastaNsis, { withFileTypes: true });
  const setups = arquivos.filter(
    (arquivo) => arquivo.isFile() && /-setup\.exe$/i.test(arquivo.name),
  );

  assert.equal(setups.length, 1, "a embalagem deve conter um único *-setup.exe");
});

test("a configuração Linux produz deb, rpm e AppImage", async () => {
  const [texto, pkgbuild] = await Promise.all([
    readFile(configuracaoLinux, "utf8"),
    readFile(receitaArch, "utf8"),
  ]);
  const configuracao = JSON.parse(texto);

  assert.deepEqual(configuracao.bundle.targets, ["deb", "rpm", "appimage"]);
  assert.ok(configuracao.bundle.linux.deb.depends.includes("libwebkit2gtk-4.1-0"));
  assert.ok(configuracao.bundle.linux.rpm.depends.includes("webkit2gtk4.1"));
  assert.match(pkgbuild, /depends=.*webkit2gtk-4\.1.*libayatana-appindicator/);
  assert.match(pkgbuild, /FOL_DISCORD_CORE_PATH=/);
});

test("a CI compila e verifica os pacotes das quatro famílias Linux", async () => {
  const workflow = await readFile(releaseWorkflow, "utf8");

  assert.match(workflow, /runs-on: ubuntu-24\.04/);
  assert.match(workflow, /cargo test --manifest-path interface\/src-tauri\/Cargo\.toml/);
  assert.match(workflow, /build --bundles deb,rpm,appimage/);
  assert.match(workflow, /verificar-pacotes\.sh/);
  assert.match(workflow, /archlinux:base-devel/);
  assert.match(workflow, /makepkg --noconfirm/);
  assert.match(workflow, /FOL-discord-x86_64\.AppImage/);
  assert.match(workflow, /GITHUB_REF_NAME[\s\S]*?v\$versao/);
  assert.match(workflow, /arch:[\s\S]*?needs: linux/);
});

test("a release pública assina pelo empacotador e verifica os artefatos", async () => {
  const workflow = await readFile(releaseWorkflow, "utf8");
  const compilarNucleo = workflow.indexOf("- name: Compilar núcleo para assinatura");
  const assinarNucleo = workflow.indexOf("- name: Assinar o núcleo antes de embutir");
  const prepararAssinatura = workflow.indexOf("- name: Preparar assinatura do empacotador");
  const buildAssinado = workflow.indexOf("- name: Build do instalador assinado");
  const testesEmbalagem = workflow.indexOf("- name: Testes da embalagem");
  const verificarAssinaturas = workflow.indexOf("- name: Verificar assinaturas dos artefatos");
  const publicarSetup = workflow.indexOf("- name: Publicar setup");
  const publicarRelease = workflow.indexOf("- name: Publicar release");

  assert.match(workflow, /cargo install artifact-signing-cli --version 0\.11\.0 --locked/);
  assert.match(workflow, /cargo build --release --locked/);
  assert.match(workflow, /AZURE_CLIENT_SECRET/);
  assert.match(workflow, /signCommand/);
  assert.match(workflow, /artifact-signing-cli/);
  assert.match(workflow, /["']%1["']/);
  assert.doesNotMatch(workflow, /azure\/artifact-signing-action/);
  assert.doesNotMatch(workflow, /build --no-bundle|tauri -- bundle/);
  assert.match(workflow, /Get-AuthenticodeSignature/);
  assert.match(workflow, /Status\s+-ne\s+["']Valid["']/);

  assert.ok(
    compilarNucleo < assinarNucleo &&
      assinarNucleo < prepararAssinatura &&
      prepararAssinatura < buildAssinado &&
      buildAssinado < testesEmbalagem &&
      testesEmbalagem < verificarAssinaturas &&
      verificarAssinaturas < publicarSetup &&
      publicarSetup < publicarRelease,
    "a release deve compilar, assinar, empacotar, verificar e só então publicar",
  );

  const blocoBuildAssinado = workflow.slice(buildAssinado, testesEmbalagem);
  assert.match(blocoBuildAssinado, /--config src-tauri\/tauri\.signing\.conf\.json/);
  assert.match(blocoBuildAssinado, /FOL_DISCORD_PREBUILT_CORE:\s*['"]?1/);
  assert.match(
    blocoBuildAssinado,
    /FOL_DISCORD_CORE_PATH:[^\n]*target\\release\\fol-discord\.exe/,
  );
  assert.match(
    workflow,
    /target\/x86_64-pc-windows-msvc\/release\/fol-discord\.exe/,
  );

  const blocoVerificacao = workflow.slice(verificarAssinaturas, publicarSetup);
  assert.ok(blocoVerificacao.includes("target\\release\\fol-discord.exe"));
  assert.ok(blocoVerificacao.includes("fol-discord-janela.exe"));
  // O sidecar e o binario que roda na maquina de quem baixa; sem ele na lista,
  // a verificacao aprova uma release cujo servico nao foi conferido.
  assert.ok(blocoVerificacao.includes("src-tauri\\binaries"));
  assert.ok(blocoVerificacao.includes("Filter '*-setup.exe'"));
});

test("a release publica o nome de setup que a janela instalada procura", async () => {
  // A janela só mostra o aviso de atualização se a release trouxer um asset com
  // o nome que o NSIS carimba. Publicar apenas a cópia de nome estável deixa
  // quem já instalou preso na versão antiga, sem aviso nenhum — foi o que
  // aconteceu na v0.2.5, e ninguém percebeu porque falha em silêncio.
  const [workflow, ponte] = await Promise.all([
    readFile(releaseWorkflow, "utf8"),
    readFile(ponteNativa, "utf8"),
  ]);

  assert.match(
    ponte,
    /format!\("FOL-discord_\{versao\}_x64-setup\.exe"\)/,
    "a janela mudou o nome que procura; ajuste a release junto",
  );
  assert.match(
    workflow,
    /files:\s*\|[\s\S]*?FOL-discord-setup\.exe[\s\S]*?bundle\/nsis\/\*-setup\.exe/,
    "a release precisa publicar o nome versionado além da cópia de nome estável",
  );
  assert.match(
    workflow,
    /\$setup\.Name -ne \$esperado/,
    "sem a trava, uma tag sem bump de versão quebra o aviso em silêncio",
  );
});

test("a janela troca o serviço antigo depois de uma atualização por cima", async () => {
  // Quando os arquivos são trocados sem passar pelo desinstalador (modo
  // silencioso, ou "Não desinstalar" na tela do instalador), o serviço antigo
  // continua rodando até o próximo logon. A janela precisa notar que a cópia
  // instalada não é mais a do instalador e trocá-la — sem passar por
  // `instalar`, que religaria o PAC de quem tinha pausado.
  const ponte = await readFile(ponteNativa, "utf8");

  assert.match(
    ponte,
    /fn garantir_servico[\s\S]*?copia_instalada_e_a_do_instalador[\s\S]*?\.arg\("rodar"\)/,
    "garantir_servico precisa trocar uma cópia desatualizada só subindo `rodar`",
  );
});

test("trazer a janela da bandeja consulta se saiu versão nova", async () => {
  // Só ao abrir e a cada seis horas não bastava: a janela passa o dia
  // escondida, e quem clicava nela logo depois de uma release não via aviso.
  // A folga entre consultas fica na ponte, para abrir-e-fechar não virar rajada.
  const [ciclo, ponte] = await Promise.all([
    readFile(cicloDeVida, "utf8"),
    readFile(ponteNativa, "utf8"),
  ]);

  assert.match(
    ciclo,
    /fn mostrar\(app: &AppHandle\) \{[\s\S]*?servico::verificar_atualizacao_ao_mostrar\(\);[\s\S]*?\n\}/,
    "mostrar() precisa disparar a consulta de atualização",
  );
  assert.match(
    ponte,
    /const FOLGA_AO_MOSTRAR: Duration = Duration::from_secs\(\d+ \* 60\);/,
    "a consulta ao mostrar precisa de folga mínima entre chamadas",
  );
});

test("a release publica soma de verificação e atestado de procedência", async () => {
  // Sem certificado de assinatura de código, são as duas únicas provas que quem
  // baixa tem de que o arquivo saiu deste repositório.
  const workflow = await readFile(releaseWorkflow, "utf8");

  assert.match(workflow, /actions\/attest-build-provenance/);
  assert.match(workflow, /id-token:\s*write/);
  assert.match(workflow, /attestations:\s*write/);
  assert.match(workflow, /SHA256SUMS\.txt/);
});

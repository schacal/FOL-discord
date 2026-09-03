use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const NUCLEO_PRECOMPILADO: &str = "FOL_DISCORD_PREBUILT_CORE";
const CAMINHO_NUCLEO: &str = "FOL_DISCORD_CORE_PATH";

fn sufixo_executavel(alvo: &str) -> &'static str {
    if alvo.contains("windows") {
        ".exe"
    } else {
        ""
    }
}

fn caminho_compilado(raiz: &Path, alvo: &str) -> PathBuf {
    raiz.join("target")
        .join(alvo)
        .join("release")
        .join(format!("fol-discord{}", sufixo_executavel(alvo)))
}

fn nucleo_precompilado(raiz: &Path, alvo: &str) -> PathBuf {
    if let Some(caminho) = env::var_os(CAMINHO_NUCLEO) {
        return PathBuf::from(caminho);
    }

    let por_alvo = caminho_compilado(raiz, alvo);
    if por_alvo.is_file() {
        return por_alvo;
    }

    // Compatibilidade com a release Windows anterior, que compilava sem
    // `--target` antes de pedir ao Tauri que usasse o núcleo já assinado.
    raiz.join("target")
        .join("release")
        .join(format!("fol-discord{}", sufixo_executavel(alvo)))
}

fn compilar_nucleo(raiz: &Path, manifesto: &Path, alvo: &str) -> PathBuf {
    let cargo = env::var_os("CARGO").expect("Cargo não foi informado pelo compilador");
    let resultado = Command::new(cargo)
        .args([
            "build",
            "--release",
            "--locked",
            "--target",
            alvo,
            "--manifest-path",
        ])
        .arg(manifesto)
        .env("CARGO_TARGET_DIR", raiz.join("target"))
        .status()
        .expect("não consegui iniciar a compilação do serviço");
    assert!(resultado.success(), "a compilação do serviço falhou");
    caminho_compilado(raiz, alvo)
}

fn main() {
    let manifesto_da_janela = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let raiz = manifesto_da_janela
        .parent()
        .and_then(|p| p.parent())
        .expect("a janela precisa continuar dentro do repositório do serviço");
    let manifesto_do_servico = raiz.join("Cargo.toml");
    let alvo = env::var("TARGET").expect("o alvo Rust não foi informado");

    let origem = if env::var(NUCLEO_PRECOMPILADO).as_deref() == Ok("1") {
        nucleo_precompilado(raiz, &alvo)
    } else {
        compilar_nucleo(raiz, &manifesto_do_servico, &alvo)
    };
    assert!(
        origem.is_file(),
        "o executável do serviço não foi encontrado em {}",
        origem.display()
    );

    let nome = format!("fol-discord{}", sufixo_executavel(&alvo));
    let destino = PathBuf::from(env::var("OUT_DIR").unwrap()).join(&nome);
    fs::copy(&origem, &destino).expect("não consegui preparar o serviço para desenvolvimento");

    // O bundler procura exatamente `nome-target-triple.ext` para cada item de
    // `externalBin`. O sufixo pertence ao alvo, não ao sistema que executa o
    // build — isso também mantém cross-compilation correta.
    let pasta_sidecar = manifesto_da_janela.join("binaries");
    fs::create_dir_all(&pasta_sidecar).expect("não consegui preparar a pasta do sidecar");
    let sidecar = pasta_sidecar.join(format!("fol-discord-{alvo}{}", sufixo_executavel(&alvo)));
    fs::copy(&origem, &sidecar).expect("não consegui preparar o sidecar do serviço");

    println!("cargo:rerun-if-changed={}", manifesto_do_servico.display());
    println!(
        "cargo:rerun-if-changed={}",
        raiz.join("Cargo.lock").display()
    );
    println!("cargo:rerun-if-changed={}", raiz.join("src").display());
    println!("cargo:rerun-if-env-changed={NUCLEO_PRECOMPILADO}");
    println!("cargo:rerun-if-env-changed={CAMINHO_NUCLEO}");
    tauri_build::build();
}

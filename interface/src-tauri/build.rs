use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    let manifesto_da_janela = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let raiz = manifesto_da_janela
        .parent()
        .and_then(|p| p.parent())
        .expect("a janela precisa continuar dentro do repositório do serviço");
    let manifesto_do_servico = raiz.join("Cargo.toml");

    // A janela é distribuída como um único .exe. Compilamos o serviço estável
    // primeiro e o copiamos para OUT_DIR, onde `include_bytes!` o embute na
    // janela sem depender de PATH, instalação anterior ou um segundo download.
    let cargo = env::var_os("CARGO").expect("Cargo não foi informado pelo compilador");
    let resultado = Command::new(cargo)
        .args(["build", "--release", "--manifest-path"])
        .arg(&manifesto_do_servico)
        .status()
        .expect("não consegui iniciar a compilação do serviço");
    assert!(resultado.success(), "a compilação do serviço falhou");

    let origem = raiz.join("target").join("release").join("fol-discord.exe");
    let destino = PathBuf::from(env::var("OUT_DIR").unwrap()).join("fol-discord.exe");
    fs::copy(&origem, &destino).expect("não consegui embutir o serviço na janela");

    // O NSIS também precisa do núcleo como sidecar: o hook de desinstalação o
    // chama antes de remover os arquivos da interface. O nome com target é o
    // padrão exigido por `bundle.externalBin` do Tauri.
    let alvo = env::var("TARGET").expect("o alvo Rust não foi informado");
    let pasta_sidecar = manifesto_da_janela.join("binaries");
    fs::create_dir_all(&pasta_sidecar).expect("não consegui preparar o sidecar do serviço");
    let sidecar = pasta_sidecar.join(format!("fol-discord-{alvo}.exe"));
    fs::copy(&origem, &sidecar).expect("não consegui preparar o sidecar do serviço");

    println!("cargo:rerun-if-changed={}", manifesto_do_servico.display());
    println!("cargo:rerun-if-changed={}", raiz.join("Cargo.lock").display());
    println!("cargo:rerun-if-changed={}", raiz.join("src").display());
    tauri_build::build();
}

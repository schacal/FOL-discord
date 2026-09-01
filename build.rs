//! Carimba identidade no executável do serviço.
//!
//! Um `.exe` sem nome, sem descrição, sem fabricante e sem ícone é o perfil que
//! os modelos de reputação de antivírus pontuam como suspeito antes mesmo de
//! olhar o que o programa faz. A janela já sai carimbada porque o Tauri escreve
//! esses campos sozinho; o serviço saía cru, e é ele que mexe no proxy — ou
//! seja, justamente o binário que mais precisa se identificar.
//!
//! O recurso é compilado pelo `rc.exe` do Windows SDK, o mesmo que o empacotador
//! do Tauri já usa. Fora do Windows não há o que carimbar.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    carimbar_identidade();
}

#[cfg(windows)]
fn carimbar_identidade() {
    const ICONE: &str = "interface/src-tauri/icones/icon.ico";

    println!("cargo:rerun-if-changed={ICONE}");

    let mut recurso = tauri_winres::WindowsResource::new();
    recurso
        .set("ProductName", "FOL-discord")
        .set("FileDescription", "Serviço do FOL-discord")
        .set("CompanyName", "schacal")
        .set("LegalCopyright", "Copyright (c) 2026 schacal")
        .set("OriginalFilename", "fol-discord.exe")
        .set("InternalName", "fol-discord");

    // O ícone vive junto da janela. Numa árvore incompleta o build continua:
    // faltar ícone é menos grave do que não compilar.
    if std::path::Path::new(ICONE).is_file() {
        recurso.set_icon(ICONE);
    } else {
        println!("cargo:warning=ícone {ICONE} ausente; o serviço sai sem ícone");
    }

    recurso
        .compile()
        .expect("não consegui carimbar o recurso de versão no serviço");
}

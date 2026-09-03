# Pacotes Linux

Os quatro alvos usam o mesmo núcleo Rust e a mesma janela Tauri:

| Família | Artefato |
| --- | --- |
| Debian / Ubuntu | `.deb` |
| Fedora / RHEL | `.rpm` |
| Arch Linux | `.pkg.tar.zst`, gerado por `arch/PKGBUILD` |
| openSUSE | `.AppImage` portátil |

O Tauri gera `deb`, `rpm` e `AppImage` com a configuração
`interface/src-tauri/tauri.linux.conf.json`. A receita Arch compila os dois
binários diretamente do tag correspondente. Em todos os formatos,
`fol-discord-janela` e o sidecar `fol-discord` ficam lado a lado.

## Compilar localmente

Instale primeiro as dependências Linux indicadas em `docs/desenvolvimento.md`.

```bash
npm ci --prefix interface
npm --prefix interface run tauri -- build --bundles deb,rpm,appimage
bash packaging/linux/verificar-pacotes.sh
```

Para validar a receita Arch dentro de uma instalação Arch:

```bash
cd packaging/arch
makepkg --syncdeps --cleanbuild
```

`pkgver` e `.SRCINFO` devem acompanhar a versão do aplicativo. A CI confere
essa igualdade antes de compilar e só produz o pacote Arch em tags, quando a
fonte `v<pkgver>` já existe.

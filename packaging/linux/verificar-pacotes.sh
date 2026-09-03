#!/usr/bin/env bash
set -euo pipefail

pasta="${1:-interface/src-tauri/target/release/bundle}"

unico() {
  local padrao="$1"
  local encontrados=()
  mapfile -t encontrados < <(find "$pasta" -type f -path "$padrao" -print)
  if [ "${#encontrados[@]}" -ne 1 ]; then
    printf 'esperava um artefato em %s, encontrei %s\n' "$padrao" "${#encontrados[@]}" >&2
    return 1
  fi
  printf '%s\n' "${encontrados[0]}"
}

deb="$(realpath "$(unico '*/deb/*.deb')")"
rpm="$(realpath "$(unico '*/rpm/*.rpm')")"
appimage="$(realpath "$(unico '*/appimage/*.AppImage')")"

conteudo_deb="$(dpkg-deb --contents "$deb")"
conteudo_rpm="$(rpm -qlp "$rpm")"
grep -Eq 'usr/bin/fol-discord-janela$' <<<"$conteudo_deb"
grep -Eq 'usr/bin/fol-discord$' <<<"$conteudo_deb"
grep -Eq 'usr/bin/fol-discord-janela$' <<<"$conteudo_rpm"
grep -Eq 'usr/bin/fol-discord$' <<<"$conteudo_rpm"

temporario="$(mktemp -d)"
trap 'rm -rf -- "$temporario"' EXIT
(
  cd "$temporario"
  "$appimage" --appimage-extract >/dev/null
)
test -x "$temporario/squashfs-root/usr/bin/fol-discord-janela"
test -x "$temporario/squashfs-root/usr/bin/fol-discord"

printf 'deb, rpm e AppImage contêm a janela e o núcleo Linux\n'

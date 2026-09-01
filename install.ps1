# desbuga-discord — instalador
#
# Uso:
#   irm https://raw.githubusercontent.com/schacal/desbuga-discord/main/install.ps1 | iex
#
# Baixa o executável mais recente publicado no GitHub, instala em
# %LOCALAPPDATA%\DesbugaDiscord e liga a correção. Não precisa de administrador.

$ErrorActionPreference = "Stop"

$repo    = "schacal/desbuga-discord"
$destino = Join-Path $env:LOCALAPPDATA "DesbugaDiscord"
$exe     = Join-Path $destino "desbuga-discord.exe"

Write-Host ""
Write-Host "desbuga-discord — instalando" -ForegroundColor Cyan
Write-Host ""

New-Item -ItemType Directory -Force -Path $destino | Out-Null

Write-Host "[1/3] Procurando a versao mais recente..."
try {
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" `
        -Headers @{ "User-Agent" = "desbuga-discord-installer" }
    $url = ($release.assets | Where-Object { $_.name -eq "desbuga-discord.exe" }).browser_download_url
    if (-not $url) { throw "release sem o executavel" }
    Write-Host "      versao $($release.tag_name)"
} catch {
    Write-Host ""
    Write-Host "Nao consegui achar uma release publicada." -ForegroundColor Red
    Write-Host "Baixe o .exe manualmente em: https://github.com/$repo/releases"
    exit 1
}

Write-Host "[2/3] Baixando..."
Invoke-WebRequest -Uri $url -OutFile $exe -UseBasicParsing

Write-Host "[3/3] Ligando a correcao..."
& $exe instalar

Write-Host ""
Write-Host "Pronto. Feche e abra o Discord uma vez." -ForegroundColor Green
Write-Host ""
Write-Host "  Ver estado:   & '$exe' status"
Write-Host "  Desinstalar:  & '$exe' desinstalar"
Write-Host ""

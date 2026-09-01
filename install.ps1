# fol-discord - instalador
#
# Uso:
#   irm https://raw.githubusercontent.com/schacal/FOL-discord/main/install.ps1 | iex
#
# Baixa o executavel mais recente publicado no GitHub, instala em
# %LOCALAPPDATA%\FolDiscord e liga a correcao. Nao precisa de administrador.

$ErrorActionPreference = "Stop"

$repo    = "schacal/FOL-discord"
$destino = Join-Path $env:LOCALAPPDATA "FolDiscord"
$exe     = Join-Path $destino "fol-discord.exe"

Write-Host ""
Write-Host "fol-discord - instalando" -ForegroundColor Cyan
Write-Host ""

New-Item -ItemType Directory -Force -Path $destino | Out-Null

Write-Host "[1/3] Procurando a versao mais recente..."
try {
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" `
        -Headers @{ "User-Agent" = "fol-discord-installer" }
    $url = ($release.assets | Where-Object { $_.name -eq "fol-discord.exe" }).browser_download_url
    if (-not $url) { throw "release sem o executavel" }
    Write-Host "      versao $($release.tag_name)"
} catch {
    Write-Host ""
    Write-Host "Nao consegui achar uma release publicada." -ForegroundColor Red
    Write-Host "Baixe o .exe manualmente em: https://github.com/$repo/releases"
    exit 1
}

Write-Host "[2/3] Baixando..."
# Baixa para um temporario primeiro: se a rede falhar no meio, a instalacao
# que ja existe continua intacta.
$tmp = "$exe.novo"
Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing

# O servico em execucao mantem o proprio .exe travado, entao ele precisa sair
# antes da troca. Sem isso, reinstalar por cima falha com "arquivo em uso".
Get-Process fol-discord -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 2
Move-Item -Path $tmp -Destination $exe -Force

Write-Host "[3/3] Ligando a correcao e reiniciando o Discord..."
# Start-Process -Wait, e nao o operador de chamada: o PowerShell nao espera
# executaveis compilados sem console, e a saida do instalador acabava saindo
# depois do prompt voltar, fora de ordem.
Start-Process -FilePath $exe -ArgumentList "instalar" -Wait -NoNewWindow

# Deixa o comando disponivel ja nesta janela; em terminais novos o PATH do
# usuario, que o instalador atualizou, resolve sozinho.
if ($env:PATH -notlike "*$destino*") { $env:PATH = "$env:PATH;$destino" }

Write-Host ""
Write-Host "Pronto." -ForegroundColor Green
Write-Host ""
Write-Host "  Ver estado:   fol-discord status"
Write-Host "  Desinstalar:  fol-discord desinstalar"
Write-Host ""

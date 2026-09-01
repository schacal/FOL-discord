# fol-discord - instalador
#
# Uso:
#   irm https://raw.githubusercontent.com/schacal/FOL-discord/main/install.ps1 | iex
#
# Baixa o instalador oficial da ultima release e o executa em modo silencioso.
# E o mesmo programa do botao de baixar do README, so que sem o assistente:
# instala no perfil do usuario, registra a desinstalacao no Windows e abre a
# janela no fim. Nao precisa de administrador.

$ErrorActionPreference = "Stop"

$repo   = "schacal/FOL-discord"
$url    = "https://github.com/$repo/releases/latest/download/FOL-discord-setup.exe"
$pacote = Join-Path $env:TEMP "FOL-discord-setup.exe"
$janela = Join-Path $env:LOCALAPPDATA "FOL-discord\fol-discord-janela.exe"

Write-Host ""
Write-Host "fol-discord - instalando" -ForegroundColor Cyan
Write-Host ""

Write-Host "[1/3] Baixando o instalador..."
# Baixa para o temporario: se a rede falhar no meio, a instalacao que ja existe
# continua intacta.
try {
    Invoke-WebRequest -Uri $url -OutFile $pacote -UseBasicParsing
} catch {
    Write-Host ""
    Write-Host "Nao consegui baixar o instalador." -ForegroundColor Red
    Write-Host "Baixe na mao em: https://github.com/$repo/releases/latest"
    exit 1
}

Write-Host "[2/3] Instalando..."
# /S e o modo silencioso do NSIS: instala sem abrir o assistente.
$setup = Start-Process -FilePath $pacote -ArgumentList "/S" -Wait -PassThru
if ($setup.ExitCode -ne 0) {
    Write-Host ""
    Write-Host "O instalador terminou com o codigo $($setup.ExitCode)." -ForegroundColor Red
    Write-Host "Rode o arquivo na mao para ver a mensagem: $pacote"
    exit 1
}
Remove-Item $pacote -Force -ErrorAction SilentlyContinue

Write-Host "[3/3] Abrindo a janela..."
# A janela e quem instala o servico embutido e liga a correcao na primeira
# abertura; o modo silencioso do NSIS nao abre nada sozinho.
if (-not (Test-Path $janela)) {
    Write-Host ""
    Write-Host "Nao encontrei a janela instalada em $janela." -ForegroundColor Red
    exit 1
}
Start-Process -FilePath $janela

Write-Host ""
Write-Host "Pronto." -ForegroundColor Green
Write-Host ""
Write-Host "  A janela cuida do resto. Fechar so a esconde na bandeja."
Write-Host "  Desinstalar:  pelo botao dentro da janela, ou em Aplicativos instalados."
Write-Host ""

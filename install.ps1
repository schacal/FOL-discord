# fol-discord - instalador
#
# Uso:
#   irm https://raw.githubusercontent.com/schacal/FOL-discord/main/install.ps1 | iex
#
# Baixa o instalador oficial da ultima release, confere o que baixou e o executa
# em modo silencioso. E o mesmo programa do botao de baixar do README, so que sem
# o assistente: instala no perfil do usuario, registra a desinstalacao no Windows
# e abre a janela no fim. Nao precisa de administrador.

$ErrorActionPreference = "Stop"

$repo   = "schacal/FOL-discord"
$api    = "https://api.github.com/repos/$repo/releases/latest"
$url    = "https://github.com/$repo/releases/latest/download/FOL-discord-setup.exe"
$pacote = Join-Path $env:TEMP "FOL-discord-setup.exe"
$janela = Join-Path $env:LOCALAPPDATA "FOL-discord\fol-discord-janela.exe"

Write-Host ""
Write-Host "fol-discord - instalando" -ForegroundColor Cyan
Write-Host ""

# A release publica o SHA-256 de cada arquivo pela propria API do GitHub. Pegar
# a soma esperada antes de baixar permite recusar um download corrompido ou
# adulterado no caminho, em vez de executa-lo e torcer.
Write-Host "[1/5] Consultando a ultima versao..."
$somaEsperada = $null
try {
    $release = Invoke-RestMethod -Uri $api -Headers @{
        "Accept"     = "application/vnd.github+json"
        "User-Agent" = "FOL-discord-install"
    } -UseBasicParsing
    $asset = $release.assets | Where-Object { $_.name -eq "FOL-discord-setup.exe" } | Select-Object -First 1
    if ($asset -and $asset.digest -match '^sha256:([0-9a-f]{64})$') {
        $somaEsperada = $Matches[1]
    }
    Write-Host "      Versao $($release.tag_name)"
} catch {
    Write-Host "      Nao consegui consultar a API do GitHub; sigo sem a soma de referencia." -ForegroundColor Yellow
}

Write-Host "[2/5] Baixando o instalador..."
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

Write-Host "[3/5] Conferindo o arquivo..."
$soma = (Get-FileHash -Algorithm SHA256 -LiteralPath $pacote).Hash.ToLower()
Write-Host "      SHA-256: $soma"

if ($somaEsperada -and $soma -ne $somaEsperada) {
    Write-Host ""
    Write-Host "O arquivo baixado nao confere com o publicado na release." -ForegroundColor Red
    Write-Host "  esperado: $somaEsperada"
    Write-Host "  recebido: $soma"
    Write-Host "Nao vou executa-lo. O arquivo ficou em: $pacote"
    exit 1
}

# Enquanto o projeto nao tem certificado de assinatura de codigo, 'NotSigned' e
# o estado normal e nao impede a instalacao. Ja 'HashMismatch' ou 'NotTrusted'
# querem dizer que alguem mexeu num arquivo que FOI assinado - ai para.
$assinatura = Get-AuthenticodeSignature -LiteralPath $pacote
switch ($assinatura.Status) {
    "Valid" {
        Write-Host "      Assinado por: $($assinatura.SignerCertificate.Subject)"
    }
    "NotSigned" {
        Write-Host "      Sem assinatura de codigo (esperado nesta versao)." -ForegroundColor Yellow
        Write-Host "      Confira a soma acima com a da pagina da release."
    }
    default {
        Write-Host ""
        Write-Host "A assinatura do instalador esta invalida (status: $($assinatura.Status))." -ForegroundColor Red
        Write-Host "Nao vou executa-lo. O arquivo suspeito ficou em: $pacote"
        exit 1
    }
}

Write-Host "[4/5] Instalando..."
# /S e o modo silencioso do NSIS: instala sem abrir o assistente.
$setup = Start-Process -FilePath $pacote -ArgumentList "/S" -Wait -PassThru
if ($setup.ExitCode -ne 0) {
    Write-Host ""
    Write-Host "O instalador terminou com o codigo $($setup.ExitCode)." -ForegroundColor Red
    Write-Host "Rode o arquivo na mao para ver a mensagem: $pacote"
    exit 1
}

Write-Host "[5/5] Abrindo a janela..."
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
Write-Host "  O instalador ficou em: $pacote"
Write-Host ""

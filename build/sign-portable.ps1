[CmdletBinding()]
param([Parameter(Mandatory)][string]$PortableRoot)

. (Join-Path $PSScriptRoot 'common.ps1')

$root = [System.IO.Path]::GetFullPath($PortableRoot)
$executable = [System.IO.Path]::GetFullPath((Join-Path $root 'GameVault.exe'))
Assert-ChildPath -Path $executable -Parent $root -Label 'portable executable'
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Portable executable is missing: $executable"
}

$pfxBase64 = [System.Environment]::GetEnvironmentVariable('GAMEVAULT_SIGNING_PFX_BASE64')
$pfxPassword = [System.Environment]::GetEnvironmentVariable('GAMEVAULT_SIGNING_PFX_PASSWORD')
$timestampUrl = [System.Environment]::GetEnvironmentVariable('GAMEVAULT_SIGNING_TIMESTAMP_URL')
$configured = @(
    @($pfxBase64, $pfxPassword, $timestampUrl) |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
)
if ($configured.Count -eq 0) {
    Write-Host 'Authenticode signing is not configured; portable output remains unsigned.' -ForegroundColor Yellow
    return
}
if ($configured.Count -ne 3) {
    throw 'Signing requires the PFX, PFX password, and RFC 3161 timestamp URL together.'
}

$timestampUri = $null
if (-not [System.Uri]::TryCreate($timestampUrl, [System.UriKind]::Absolute, [ref]$timestampUri) -or
    $timestampUri.Scheme -ne [System.Uri]::UriSchemeHttps) {
    throw 'The signing timestamp URL must be an absolute HTTPS URL.'
}

$signTool = Get-Command 'signtool.exe' -ErrorAction SilentlyContinue |
    Select-Object -First 1 -ExpandProperty Source
if (-not $signTool) {
    $kitsRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (Test-Path -LiteralPath $kitsRoot -PathType Container) {
        $signTool = Get-ChildItem -LiteralPath $kitsRoot -Directory |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName 'x64\signtool.exe' } |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
            Select-Object -First 1
    }
}
if (-not $signTool) {
    throw 'signtool.exe was not found in PATH or the Windows 10 SDK.'
}

$systemTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
$signingTemp = [System.IO.Path]::GetFullPath(
    (Join-Path $systemTemp "gamevault-signing-$([guid]::NewGuid().ToString('N'))")
)
Assert-ChildPath -Path $signingTemp -Parent $systemTemp -Label 'signing temporary directory'
New-Item -ItemType Directory -Path $signingTemp | Out-Null
$pfxPath = Join-Path $signingTemp 'certificate.pfx'
$pfxBytes = $null
try {
    try {
        $pfxBytes = [System.Convert]::FromBase64String($pfxBase64)
    }
    catch {
        throw 'The configured signing certificate is not valid base64.'
    }
    [System.IO.File]::WriteAllBytes($pfxPath, $pfxBytes)
    & $signTool sign /fd SHA256 /td SHA256 /tr $timestampUri.AbsoluteUri /f $pfxPath /p $pfxPassword $executable
    if ($LASTEXITCODE -ne 0) {
        throw "Authenticode signing failed with exit code $LASTEXITCODE."
    }
    & $signTool verify /pa /all $executable
    if ($LASTEXITCODE -ne 0) {
        throw "Authenticode signature verification failed with exit code $LASTEXITCODE."
    }
}
finally {
    $pfxBytes = $null
    $pfxPassword = $null
    if (Test-Path -LiteralPath $signingTemp -PathType Container) {
        Remove-Item -LiteralPath $signingTemp -Recurse -Force
    }
}

Write-Host 'GameVault.exe was Authenticode-signed and verified.' -ForegroundColor Green

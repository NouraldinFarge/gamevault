[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ProjectRoot,
    [Parameter(Mandatory)][string]$StageRoot,
    [Parameter(Mandatory)][string]$ArchivePath,
    [Parameter(Mandatory)][string]$Version
)

. (Join-Path $PSScriptRoot 'common.ps1')

$project = [System.IO.Path]::GetFullPath($ProjectRoot)
$stage = [System.IO.Path]::GetFullPath($StageRoot)
$archive = [System.IO.Path]::GetFullPath($ArchivePath)
Assert-ChildPath -Path $archive -Parent (Join-Path $project 'portable-builds') -Label 'portable archive'
if (Test-Path -LiteralPath $archive) {
    throw "Portable archive already exists and will not be overwritten: $archive"
}

New-Item -ItemType Directory -Path (Split-Path -Parent $archive) -Force | Out-Null
Compress-Archive -LiteralPath $stage -DestinationPath $archive -CompressionLevel Optimal

$verifyRoot = Join-Path $project "workspace\temp\archive-verify-$([guid]::NewGuid().ToString('N'))"
Assert-ChildPath -Path $verifyRoot -Parent (Join-Path $project 'workspace\temp') -Label 'archive verification root'
New-Item -ItemType Directory -Path $verifyRoot -Force | Out-Null
try {
    Expand-Archive -LiteralPath $archive -DestinationPath $verifyRoot
    $wrapper = @(Get-ChildItem -LiteralPath $verifyRoot -Directory)
    if ($wrapper.Count -ne 1) {
        throw 'Portable ZIP must contain exactly one versioned top-level folder.'
    }
    & (Join-Path $PSScriptRoot 'verify-portable-build.ps1') `
        -PortableRoot $wrapper[0].FullName `
        -ExpectedVersion $Version
}
finally {
    if (Test-Path -LiteralPath $verifyRoot) {
        Remove-Item -LiteralPath $verifyRoot -Recurse -Force
    }
}

$checksumPath = Join-Path $project "checksums\$([System.IO.Path]::GetFileName($archive)).sha256"
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  $([System.IO.Path]::GetFileName($archive))" | Set-Content -LiteralPath $checksumPath -Encoding ascii
Write-Host 'Portable ZIP and checksum created.' -ForegroundColor Green

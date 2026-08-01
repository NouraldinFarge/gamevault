[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ProjectRoot,
    [Parameter(Mandatory)][string]$ArchivePath,
    [Parameter(Mandatory)][string]$Version
)

. (Join-Path $PSScriptRoot 'common.ps1')

$project = [System.IO.Path]::GetFullPath($ProjectRoot)
$active = Join-Path $project 'active-build'
$archive = [System.IO.Path]::GetFullPath($ArchivePath)
$metadata = [ordered]@{
    schemaVersion = '1.0'
    application = 'GameVault'
    version = $Version
    channel = 'portable'
    targetOperatingSystem = 'windows'
    targetArchitecture = 'x64'
    createdAt = [DateTime]::UtcNow.ToString('o')
    archivePath = $archive
    archiveSha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    activeBuildPath = $active
    activeLayout = 'flat'
    deploymentPolicy = 'verified transactional replacement with data carry-forward and rollback'
    installerGenerated = $false
    webView2Policy = 'Windows-provided Evergreen runtime'
}
$metadataDirectory = [System.IO.Path]::GetFullPath((Join-Path $project 'release-metadata'))
Assert-ChildPath -Path $metadataDirectory -Parent $project -Label 'release metadata directory'
New-Item -ItemType Directory -Path $metadataDirectory -Force | Out-Null
$path = Join-Path $metadataDirectory "GameVault-v$Version-windows-x64-portable.json"
if (Test-Path -LiteralPath $path) {
    throw "Release metadata already exists and will not be overwritten: $path"
}
$metadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $path -Encoding utf8
Write-Host 'Release metadata written.' -ForegroundColor Green

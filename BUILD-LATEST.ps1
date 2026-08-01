[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$workspaceRoot = [System.IO.Path]::GetFullPath($PSScriptRoot)
$projectRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $workspaceRoot))
$activeBuild = [System.IO.Path]::GetFullPath((Join-Path $projectRoot 'active-build'))

if ((Split-Path -Parent $activeBuild) -ne $projectRoot) {
    throw 'Architectural validation failed: active-build must be a direct child of the project root.'
}

$version = (Get-Content -Raw -LiteralPath (Join-Path $workspaceRoot 'VERSION')).Trim()
if ($version -notmatch '^\d+\.\d+\.\d+([-.][0-9A-Za-z.-]+)?$') {
    throw "VERSION is not a semantic version: $version"
}

$releaseName = "GameVault-v$version-windows-x64-portable"
$stagingRoot = Join-Path $workspaceRoot "output\$releaseName"
$archivePath = Join-Path $projectRoot "portable-builds\$releaseName.zip"

try {
    Write-Host ''
    Write-Host "GameVault $version portable build" -ForegroundColor Cyan
    Write-Host 'Installer generation is disabled by project policy.' -ForegroundColor DarkGray

    & (Join-Path $workspaceRoot 'build\validate-environment.ps1') -WorkspaceRoot $workspaceRoot
    & (Join-Path $workspaceRoot 'build\clean-output.ps1') -WorkspaceRoot $workspaceRoot
    & (Join-Path $workspaceRoot 'build\run-tests.ps1') -WorkspaceRoot $workspaceRoot
    & (Join-Path $workspaceRoot 'build\build-portable.ps1') `
        -WorkspaceRoot $workspaceRoot `
        -StageRoot $stagingRoot `
        -Version $version
    & (Join-Path $workspaceRoot 'build\verify-portable-build.ps1') `
        -PortableRoot $stagingRoot `
        -ExpectedVersion $version
    & (Join-Path $workspaceRoot 'build\package-portable.ps1') `
        -ProjectRoot $projectRoot `
        -StageRoot $stagingRoot `
        -ArchivePath $archivePath `
        -Version $version
    & (Join-Path $workspaceRoot 'build\deploy-active-build.ps1') `
        -WorkspaceRoot $workspaceRoot `
        -ProjectRoot $projectRoot `
        -ArchivePath $archivePath `
        -Version $version
    & (Join-Path $workspaceRoot 'build\write-release-metadata.ps1') `
        -ProjectRoot $projectRoot `
        -ArchivePath $archivePath `
        -Version $version
    & (Join-Path $workspaceRoot 'build\clean-output.ps1') -WorkspaceRoot $workspaceRoot

    Write-Host ''
    Write-Host 'Portable build completed successfully.' -ForegroundColor Green
    Write-Host "Archive:      $archivePath"
    Write-Host "Active build: $activeBuild"
    exit 0
}
catch {
    Write-Host ''
    Write-Host 'Portable build failed.' -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host 'The previous active-build was preserved or restored.' -ForegroundColor Yellow
    exit 1
}

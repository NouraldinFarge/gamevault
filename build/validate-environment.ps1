[CmdletBinding()]
param([Parameter(Mandatory)][string]$WorkspaceRoot)

. (Join-Path $PSScriptRoot 'common.ps1')

$workspace = [System.IO.Path]::GetFullPath($WorkspaceRoot)
$requiredFiles = @(
    'VERSION',
    'package.json',
    'pnpm-lock.yaml',
    'config\default-settings.json',
    'docs\PORTABLE-README.txt',
    'docs\RUNTIME-README.txt',
    'assets\README.txt',
    'LICENSE',
    'src-tauri\Cargo.toml',
    'src-tauri\tauri.conf.json',
    'src\main.tsx'
)
foreach ($file in $requiredFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $workspace $file) -PathType Leaf)) {
        throw "Required build file is missing: $file"
    }
}

$defaultSettingsPath = Join-Path $workspace 'config\default-settings.json'
$defaultSettings = Get-Content -Raw -LiteralPath $defaultSettingsPath | ConvertFrom-Json
if ([System.IO.Path]::IsPathRooted([string]$defaultSettings.managedRoot)) {
    throw 'Portable default managedRoot must be relative to the application directory.'
}
foreach ($libraryRoot in @($defaultSettings.libraryRoots)) {
    if ([System.IO.Path]::IsPathRooted([string]$libraryRoot)) {
        throw 'Portable default libraryRoots entries must be relative to the application directory.'
    }
}

foreach ($tool in @('node', 'pnpm', 'cargo', 'rustc')) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "Required build tool '$tool' is missing from PATH."
    }
}

$version = (Get-Content -Raw -LiteralPath (Join-Path $workspace 'VERSION')).Trim()
Test-DerivedVersion -WorkspaceRoot $workspace -Version $version

$cargoVersion = & cargo --version
$rustVersion = & rustc --version
$nodeVersion = & node --version
$pnpmVersion = & pnpm --version
Write-Host 'Environment validated.' -ForegroundColor Green
Write-Host "  $nodeVersion; pnpm $pnpmVersion"
Write-Host "  $rustVersion; $cargoVersion"

[CmdletBinding()]
param([Parameter(Mandatory)][string]$WorkspaceRoot)

. (Join-Path $PSScriptRoot 'common.ps1')

$workspace = [System.IO.Path]::GetFullPath($WorkspaceRoot)
$requiredFiles = @(
    'VERSION',
    'package.json',
    'pnpm-lock.yaml',
    'src-tauri\Cargo.toml',
    'src-tauri\tauri.conf.json',
    'src\main.tsx'
)
foreach ($file in $requiredFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $workspace $file) -PathType Leaf)) {
        throw "Required build file is missing: $file"
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


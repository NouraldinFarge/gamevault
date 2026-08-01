[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$WorkspaceRoot,
    [Parameter(Mandatory)][string]$StageRoot,
    [Parameter(Mandatory)][string]$Version
)

. (Join-Path $PSScriptRoot 'common.ps1')

$workspace = [System.IO.Path]::GetFullPath($WorkspaceRoot)
$stage = [System.IO.Path]::GetFullPath($StageRoot)
Assert-ChildPath -Path $stage -Parent (Join-Path $workspace 'output') -Label 'portable staging root'

Invoke-Checked `
    -FilePath 'pnpm' `
    -Arguments @('build') `
    -WorkingDirectory $workspace `
    -Description 'Build the production interface'
Invoke-Checked `
    -FilePath 'cargo' `
    -Arguments @(
        'build',
        '--release',
        '--locked',
        '--features',
        'tauri/custom-protocol',
        '--manifest-path',
        'src-tauri\Cargo.toml'
    ) `
    -WorkingDirectory $workspace `
    -Description 'Build the portable Tauri executable with embedded production assets'

New-Item -ItemType Directory -Path $stage -Force | Out-Null
foreach ($directory in @('assets', 'config', 'data', 'logs', 'cache', 'runtime', 'licenses')) {
    New-Item -ItemType Directory -Path (Join-Path $stage $directory) -Force | Out-Null
}

$executable = Join-Path $workspace 'src-tauri\target\release\GameVault.exe'
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw 'Cargo completed but GameVault.exe was not produced.'
}
Copy-Item -LiteralPath $executable -Destination (Join-Path $stage 'GameVault.exe')
Copy-Item -LiteralPath (Join-Path $workspace 'VERSION') -Destination (Join-Path $stage 'VERSION')
Copy-Item -LiteralPath (Join-Path $workspace 'docs\PORTABLE-README.txt') -Destination (Join-Path $stage 'README.txt')
Copy-Item -LiteralPath (Join-Path $workspace 'config\default-settings.json') -Destination (Join-Path $stage 'config\default-settings.json')
Copy-Item -LiteralPath (Join-Path $workspace 'LICENSE') -Destination (Join-Path $stage 'licenses\LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $workspace 'assets\README.txt') -Destination (Join-Path $stage 'assets\README.txt')
Copy-Item -LiteralPath (Join-Path $workspace 'docs\RUNTIME-README.txt') -Destination (Join-Path $stage 'runtime\README.txt')

Write-Host "Portable staging created for version $Version." -ForegroundColor Green

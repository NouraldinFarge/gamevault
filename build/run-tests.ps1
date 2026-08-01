[CmdletBinding()]
param([Parameter(Mandatory)][string]$WorkspaceRoot)

. (Join-Path $PSScriptRoot 'common.ps1')

$workspace = [System.IO.Path]::GetFullPath($WorkspaceRoot)
if (-not (Test-Path -LiteralPath (Join-Path $workspace 'node_modules'))) {
    Invoke-Checked `
        -FilePath 'pnpm' `
        -Arguments @('install', '--frozen-lockfile') `
        -WorkingDirectory $workspace `
        -Description 'Restore locked frontend dependencies'
}
Invoke-Checked `
    -FilePath 'pnpm' `
    -Arguments @('check') `
    -WorkingDirectory $workspace `
    -Description 'Frontend formatting, lint, and type validation'
Invoke-Checked `
    -FilePath 'pnpm' `
    -Arguments @('test') `
    -WorkingDirectory $workspace `
    -Description 'Frontend automated tests'
Invoke-Checked `
    -FilePath 'cargo' `
    -Arguments @('test', '--locked', '--manifest-path', 'src-tauri\Cargo.toml') `
    -WorkingDirectory $workspace `
    -Description 'Rust unit and storage tests'


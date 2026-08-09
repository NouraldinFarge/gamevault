[CmdletBinding()]
param([Parameter(Mandatory)][string]$WorkspaceRoot)

. (Join-Path $PSScriptRoot 'common.ps1')

$workspace = [System.IO.Path]::GetFullPath($WorkspaceRoot)
Invoke-Checked `
    -FilePath 'pnpm' `
    -Arguments @('install', '--frozen-lockfile') `
    -WorkingDirectory $workspace `
    -Description 'Restore locked frontend dependencies'
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
    -Arguments @('fmt', '--manifest-path', 'src-tauri\Cargo.toml', '--all', '--check') `
    -WorkingDirectory $workspace `
    -Description 'Rust formatting validation'
Invoke-Checked `
    -FilePath 'cargo' `
    -Arguments @(
        'clippy',
        '--manifest-path',
        'src-tauri\Cargo.toml',
        '--all-targets',
        '--locked',
        '--',
        '-D',
        'warnings'
    ) `
    -WorkingDirectory $workspace `
    -Description 'Rust lint validation'
Invoke-Checked `
    -FilePath 'cargo' `
    -Arguments @('test', '--locked', '--manifest-path', 'src-tauri\Cargo.toml') `
    -WorkingDirectory $workspace `
    -Description 'Rust unit and storage tests'
Invoke-Checked `
    -FilePath 'pwsh' `
    -Arguments @('-NoProfile', '-File', 'build\test-portable-scripts.ps1', '-WorkspaceRoot', $workspace) `
    -WorkingDirectory $workspace `
    -Description 'Portable upgrade state-preservation tests'

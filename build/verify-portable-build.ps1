[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PortableRoot,
    [Parameter(Mandatory)][string]$ExpectedVersion
)

. (Join-Path $PSScriptRoot 'common.ps1')

$root = [System.IO.Path]::GetFullPath($PortableRoot)
foreach ($required in @(
    'GameVault.exe',
    'VERSION',
    'README.txt',
    'assets',
    'config',
    'data',
    'logs',
    'cache',
    'runtime',
    'licenses'
)) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $required))) {
        throw "Portable build is missing: $required"
    }
}

$version = (Get-Content -Raw -LiteralPath (Join-Path $root 'VERSION')).Trim()
if ($version -ne $ExpectedVersion) {
    throw "Portable VERSION $version does not match expected version $ExpectedVersion."
}

$forbidden = Get-ChildItem -LiteralPath $root -Recurse -Force |
    Where-Object {
        $_.Name -match '(?i)(setup.*\.exe$|uninstall|\.msi$|\.msix$|\.appx$|\.dmg$|\.pkg$|nsis|inno)'
    }
if ($forbidden) {
    throw "Installer artifact detected in portable output: $($forbidden[0].FullName)"
}

if (Test-Path -LiteralPath (Join-Path $root 'active-build')) {
    throw 'Portable output contains an invalid nested active-build directory.'
}

Invoke-PortableHealthCheck -Executable (Join-Path $root 'GameVault.exe')
Write-Host 'Portable layout and executable health verified.' -ForegroundColor Green


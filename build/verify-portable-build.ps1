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
$probe = Join-Path (Split-Path -Parent $root) "GameVault renamed path $([guid]::NewGuid().ToString('N'))"
Assert-ChildPath -Path $probe -Parent (Split-Path -Parent $root) -Label 'renamed portable probe'
try {
    New-Item -ItemType Directory -Path $probe -Force | Out-Null
    $renamedExecutable = Join-Path $probe 'Renamed GameVault.exe'
    Copy-Item -LiteralPath (Join-Path $root 'GameVault.exe') -Destination $renamedExecutable
    Invoke-PortableHealthCheck -Executable $renamedExecutable
    if (-not (Test-Path -LiteralPath (Join-Path $probe 'data\library.db') -PathType Leaf)) {
        throw 'Renamed portable health check wrote its database outside the executable folder.'
    }
}
finally {
    if (Test-Path -LiteralPath $probe) {
        Remove-Item -LiteralPath $probe -Recurse -Force
    }
}
Write-Host 'Portable layout and executable health verified.' -ForegroundColor Green

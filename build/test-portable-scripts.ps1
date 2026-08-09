[CmdletBinding()]
param([Parameter(Mandatory)][string]$WorkspaceRoot)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

$workspace = [System.IO.Path]::GetFullPath($WorkspaceRoot)
$testRoot = Join-Path $workspace "temp\portable-state-test-$([guid]::NewGuid().ToString('N'))"
Assert-ChildPath -Path $testRoot -Parent (Join-Path $workspace 'temp') -Label 'portable state test root'
$active = Join-Path $testRoot 'old build'
$next = Join-Path $testRoot 'renamed portable build'

try {
    foreach ($path in @(
        (Join-Path $active 'data\backups'),
        (Join-Path $active 'library\Games\Owned Game'),
        (Join-Path $active 'library\Inbox'),
        (Join-Path $active 'logs'),
        (Join-Path $active 'config'),
        (Join-Path $next 'config')
    )) {
        New-Item -ItemType Directory -Path $path -Force | Out-Null
    }
    Set-Content -LiteralPath (Join-Path $active 'data\library.db') -Value 'database'
    Set-Content -LiteralPath (Join-Path $active 'data\backups\backup.db') -Value 'backup'
    Set-Content -LiteralPath (Join-Path $active 'library\Games\Owned Game\Game.exe') -Value 'game'
    Set-Content -LiteralPath (Join-Path $active 'library\Inbox\Owned Game.zip') -Value 'archive'
    Set-Content -LiteralPath (Join-Path $active 'logs\gamevault.log') -Value 'log'
    Set-Content -LiteralPath (Join-Path $active 'config\settings.json') -Value 'user settings'
    Set-Content -LiteralPath (Join-Path $active 'config\default-settings.json') -Value 'old defaults'
    Set-Content -LiteralPath (Join-Path $next 'config\default-settings.json') -Value 'new defaults'

    Copy-PortableUserState -ActiveRoot $active -NextRoot $next
    $libraryMoved = Move-PortableLibraryState -PreviousRoot $active -NextRoot $next
    if (-not $libraryMoved) {
        throw 'The portable managed library was not transferred.'
    }

    foreach ($relative in @(
        'data\library.db',
        'data\backups\backup.db',
        'library\Games\Owned Game\Game.exe',
        'library\Inbox\Owned Game.zip',
        'logs\gamevault.log',
        'config\settings.json'
    )) {
        if (-not (Test-Path -LiteralPath (Join-Path $next $relative) -PathType Leaf)) {
            throw "Portable state was not preserved: $relative"
        }
    }
    $defaults = (Get-Content -Raw -LiteralPath (Join-Path $next 'config\default-settings.json')).Trim()
    if ($defaults -ne 'new defaults') {
        throw 'The new release defaults were overwritten by the previous build.'
    }
}
finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}

Write-Host 'Portable upgrade state preservation verified.' -ForegroundColor Green

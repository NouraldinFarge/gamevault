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

    $zipSource = Join-Path $testRoot 'deterministic source'
    New-Item -ItemType Directory -Path (Join-Path $zipSource 'empty') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $zipSource 'nested') -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $zipSource 'z-last.txt') -Value 'last'
    Set-Content -LiteralPath (Join-Path $zipSource 'nested\a-first.txt') -Value 'first'
    $zipA = Join-Path $testRoot 'first.zip'
    $zipB = Join-Path $testRoot 'second.zip'
    New-DeterministicPortableZip -SourceDirectory $zipSource -ArchivePath $zipA
    New-DeterministicPortableZip -SourceDirectory $zipSource -ArchivePath $zipB
    $hashA = (Get-FileHash -LiteralPath $zipA -Algorithm SHA256).Hash
    $hashB = (Get-FileHash -LiteralPath $zipB -Algorithm SHA256).Hash
    if ($hashA -ne $hashB) {
        throw 'Identical portable inputs did not produce byte-identical ZIP archives.'
    }
    $zipStream = [System.IO.File]::OpenRead($zipA)
    try {
        $zip = [System.IO.Compression.ZipArchive]::new(
            $zipStream,
            [System.IO.Compression.ZipArchiveMode]::Read,
            $false
        )
        try {
            $unexpectedTimestamp = $zip.Entries | Where-Object {
                $_.LastWriteTime.DateTime -ne [datetime]'2000-01-01T00:00:00'
            } | Select-Object -First 1
            if ($unexpectedTimestamp) {
                throw "Portable ZIP timestamp was not normalized: $($unexpectedTimestamp.FullName)"
            }
        }
        finally {
            $zip.Dispose()
        }
    }
    finally {
        $zipStream.Dispose()
    }

    $signingProbe = Join-Path $testRoot 'signing probe'
    New-Item -ItemType Directory -Path $signingProbe -Force | Out-Null
    Set-Content -LiteralPath (Join-Path $signingProbe 'GameVault.exe') -Value 'unsigned fixture'
    $signingVariables = @(
        'GAMEVAULT_SIGNING_PFX_BASE64',
        'GAMEVAULT_SIGNING_PFX_PASSWORD',
        'GAMEVAULT_SIGNING_TIMESTAMP_URL'
    )
    $originalSigningValues = @{}
    foreach ($name in $signingVariables) {
        $originalSigningValues[$name] = [System.Environment]::GetEnvironmentVariable($name)
        [System.Environment]::SetEnvironmentVariable($name, $null)
    }
    try {
        $unsignedHash = (Get-FileHash -LiteralPath (Join-Path $signingProbe 'GameVault.exe') -Algorithm SHA256).Hash
        & (Join-Path $PSScriptRoot 'sign-portable.ps1') -PortableRoot $signingProbe
        $afterNoOpHash = (Get-FileHash -LiteralPath (Join-Path $signingProbe 'GameVault.exe') -Algorithm SHA256).Hash
        if ($unsignedHash -ne $afterNoOpHash) {
            throw 'The unconfigured signing hook changed the portable executable.'
        }
        [System.Environment]::SetEnvironmentVariable('GAMEVAULT_SIGNING_PFX_BASE64', 'ZHVtbXk=')
        $partialConfigurationFailed = $false
        try {
            & (Join-Path $PSScriptRoot 'sign-portable.ps1') -PortableRoot $signingProbe
        }
        catch {
            $partialConfigurationFailed = $true
        }
        if (-not $partialConfigurationFailed) {
            throw 'A partial signing configuration did not fail closed.'
        }
    }
    finally {
        foreach ($name in $signingVariables) {
            [System.Environment]::SetEnvironmentVariable($name, $originalSigningValues[$name])
        }
    }
}
finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}

Write-Host 'Portable state, deterministic ZIP, and signing-hook behavior verified.' -ForegroundColor Green

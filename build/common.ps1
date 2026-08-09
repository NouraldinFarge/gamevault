Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-ChildPath {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Parent,
        [Parameter(Mandatory)][string]$Label
    )
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    $resolvedParent = [System.IO.Path]::GetFullPath($Parent).TrimEnd('\') + '\'
    if (-not $resolvedPath.StartsWith($resolvedParent, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "$Label is outside the approved root."
    }
    if ($resolvedPath.TrimEnd('\') -eq $resolvedParent.TrimEnd('\')) {
        throw "$Label must not equal its approved parent."
    }
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][string]$Description
    )
    Write-Host "  -> $Description" -ForegroundColor DarkCyan
    Push-Location $WorkingDirectory
    try {
        & $FilePath @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "$Description failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }
}

function Test-DerivedVersion {
    param(
        [Parameter(Mandatory)][string]$WorkspaceRoot,
        [Parameter(Mandatory)][string]$Version
    )
    $package = Get-Content -Raw -LiteralPath (Join-Path $WorkspaceRoot 'package.json') | ConvertFrom-Json
    if ($package.version -ne $Version) {
        throw "package.json version $($package.version) does not match authoritative VERSION $Version."
    }
    $cargo = Get-Content -Raw -LiteralPath (Join-Path $WorkspaceRoot 'src-tauri\Cargo.toml')
    if ($cargo -notmatch "(?m)^version = `"$([regex]::Escape($Version))`"$") {
        throw 'Cargo.toml version does not match the authoritative VERSION file.'
    }
    $tauri = Get-Content -Raw -LiteralPath (Join-Path $WorkspaceRoot 'src-tauri\tauri.conf.json') | ConvertFrom-Json
    if ($tauri.version -ne $Version) {
        throw 'tauri.conf.json version does not match the authoritative VERSION file.'
    }
}

function Invoke-PortableHealthCheck {
    param([Parameter(Mandatory)][string]$Executable)
    if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
        throw "Portable executable is missing: $Executable"
    }
    $process = Start-Process `
        -FilePath $Executable `
        -ArgumentList '--health-check' `
        -PassThru `
        -Wait `
        -WindowStyle Hidden
    if ($process.ExitCode -ne 0) {
        throw "Portable executable health check failed with exit code $($process.ExitCode)."
    }
}

function Assert-NoReparsePoints {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Label
    )
    if (-not (Test-Path -LiteralPath $Path)) {
        return
    }
    $root = Get-Item -LiteralPath $Path -Force
    $entries = @($root) + @(Get-ChildItem -LiteralPath $Path -Recurse -Force)
    $reparse = $entries | Where-Object {
        ($_.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0
    } | Select-Object -First 1
    if ($reparse) {
        throw "$Label contains a link or Windows reparse point: $($reparse.FullName)"
    }
}

function Copy-DirectoryContents {
    param(
        [Parameter(Mandatory)][string]$Source,
        [Parameter(Mandatory)][string]$Destination,
        [string[]]$ExcludedNames = @()
    )
    if (-not (Test-Path -LiteralPath $Source -PathType Container)) {
        return
    }
    Assert-NoReparsePoints -Path $Source -Label 'Portable user state'
    New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    foreach ($entry in Get-ChildItem -LiteralPath $Source -Force) {
        if ($ExcludedNames -contains $entry.Name) {
            continue
        }
        Copy-Item -LiteralPath $entry.FullName -Destination $Destination -Recurse -Force
    }
}

function Copy-PortableUserState {
    param(
        [Parameter(Mandatory)][string]$ActiveRoot,
        [Parameter(Mandatory)][string]$NextRoot
    )
    $active = [System.IO.Path]::GetFullPath($ActiveRoot)
    $next = [System.IO.Path]::GetFullPath($NextRoot)
    if ($active -eq $next) {
        throw 'Portable upgrade source and destination must be different.'
    }
    foreach ($directory in @('data', 'logs')) {
        Copy-DirectoryContents `
            -Source (Join-Path $active $directory) `
            -Destination (Join-Path $next $directory)
    }
    Copy-DirectoryContents `
        -Source (Join-Path $active 'config') `
        -Destination (Join-Path $next 'config') `
        -ExcludedNames @('default-settings.json')
}

function Move-PortableLibraryState {
    param(
        [Parameter(Mandatory)][string]$PreviousRoot,
        [Parameter(Mandatory)][string]$NextRoot
    )
    $source = Join-Path ([System.IO.Path]::GetFullPath($PreviousRoot)) 'library'
    $destination = Join-Path ([System.IO.Path]::GetFullPath($NextRoot)) 'library'
    if (-not (Test-Path -LiteralPath $source -PathType Container)) {
        return $false
    }
    $sourceItem = Get-Item -LiteralPath $source -Force
    if (($sourceItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
        throw "Portable managed library is a link or Windows reparse point: $source"
    }
    if (Test-Path -LiteralPath $destination) {
        $entries = @(Get-ChildItem -LiteralPath $destination -Force)
        if ($entries.Count -ne 0) {
            throw 'The next portable build already contains managed library data.'
        }
        Remove-Item -LiteralPath $destination -Force
    }
    Move-Item -LiteralPath $source -Destination $destination
    return $true
}

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


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
    $releaseStatus = Get-Content -Raw -LiteralPath (Join-Path $WorkspaceRoot 'release-status.json') | ConvertFrom-Json
    if ($releaseStatus.sourceVersion -ne $Version) {
        throw 'release-status.json sourceVersion does not match the authoritative VERSION file.'
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

function New-DeterministicPortableZip {
    param(
        [Parameter(Mandatory)][string]$SourceDirectory,
        [Parameter(Mandatory)][string]$ArchivePath
    )
    $source = [System.IO.Path]::GetFullPath($SourceDirectory).TrimEnd('\')
    $destination = [System.IO.Path]::GetFullPath($ArchivePath)
    if (-not (Test-Path -LiteralPath $source -PathType Container)) {
        throw "Portable ZIP source directory is missing: $source"
    }
    if (Test-Path -LiteralPath $destination) {
        throw "Portable archive already exists and will not be overwritten: $destination"
    }
    Assert-NoReparsePoints -Path $source -Label 'Portable ZIP source'
    New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null

    Add-Type -AssemblyName System.IO.Compression
    $rootName = Split-Path -Leaf $source
    $directoryNames = [System.Collections.Generic.List[string]]::new()
    $directoryNames.Add("$rootName/")
    foreach ($directory in Get-ChildItem -LiteralPath $source -Directory -Recurse -Force) {
        $relative = $directory.FullName.Substring($source.Length).TrimStart('\').Replace('\', '/')
        $directoryNames.Add("$rootName/$relative/")
    }
    $sortedDirectories = $directoryNames.ToArray()
    [System.Array]::Sort($sortedDirectories, [System.StringComparer]::Ordinal)

    $filesByName = [System.Collections.Generic.Dictionary[string, string]]::new(
        [System.StringComparer]::Ordinal
    )
    foreach ($file in Get-ChildItem -LiteralPath $source -File -Recurse -Force) {
        $relative = $file.FullName.Substring($source.Length).TrimStart('\').Replace('\', '/')
        $filesByName.Add("$rootName/$relative", $file.FullName)
    }
    $sortedFiles = [string[]]@($filesByName.Keys)
    [System.Array]::Sort($sortedFiles, [System.StringComparer]::Ordinal)

    $fixedTimestamp = [System.DateTimeOffset]::new(
        2000, 1, 1, 0, 0, 0, [System.TimeSpan]::Zero
    )
    $archiveStream = [System.IO.File]::Open(
        $destination,
        [System.IO.FileMode]::CreateNew,
        [System.IO.FileAccess]::Write,
        [System.IO.FileShare]::None
    )
    try {
        $zip = [System.IO.Compression.ZipArchive]::new(
            $archiveStream,
            [System.IO.Compression.ZipArchiveMode]::Create,
            $false
        )
        try {
            foreach ($entryName in $sortedDirectories) {
                $entry = $zip.CreateEntry(
                    $entryName,
                    [System.IO.Compression.CompressionLevel]::Optimal
                )
                $entry.LastWriteTime = $fixedTimestamp
                $entry.ExternalAttributes = 16
            }
            foreach ($entryName in $sortedFiles) {
                $entry = $zip.CreateEntry(
                    $entryName,
                    [System.IO.Compression.CompressionLevel]::Optimal
                )
                $entry.LastWriteTime = $fixedTimestamp
                $entry.ExternalAttributes = 0
                $inputStream = [System.IO.File]::Open(
                    $filesByName[$entryName],
                    [System.IO.FileMode]::Open,
                    [System.IO.FileAccess]::Read,
                    [System.IO.FileShare]::Read
                )
                try {
                    $outputStream = $entry.Open()
                    try {
                        $inputStream.CopyTo($outputStream)
                    }
                    finally {
                        $outputStream.Dispose()
                    }
                }
                finally {
                    $inputStream.Dispose()
                }
            }
        }
        finally {
            $zip.Dispose()
        }
    }
    catch {
        $archiveStream.Dispose()
        if (Test-Path -LiteralPath $destination -PathType Leaf) {
            [System.IO.File]::Delete($destination)
        }
        throw
    }
    finally {
        $archiveStream.Dispose()
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

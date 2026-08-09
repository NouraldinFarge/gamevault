[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$WorkspaceRoot,
    [Parameter(Mandatory)][string]$ProjectRoot,
    [Parameter(Mandatory)][string]$ArchivePath,
    [Parameter(Mandatory)][string]$Version
)

. (Join-Path $PSScriptRoot 'common.ps1')

$workspace = [System.IO.Path]::GetFullPath($WorkspaceRoot)
$project = [System.IO.Path]::GetFullPath($ProjectRoot)
$active = [System.IO.Path]::GetFullPath((Join-Path $project 'active-build'))
if ((Split-Path -Parent $active) -ne $project) {
    throw 'active-build is not a direct child of the project root.'
}

$deployment = Join-Path $workspace "temp\deployment-$([guid]::NewGuid().ToString('N'))"
$backup = Join-Path $workspace "temp\previous-active-$([guid]::NewGuid().ToString('N'))"
Assert-ChildPath -Path $deployment -Parent (Join-Path $workspace 'temp') -Label 'deployment root'
Assert-ChildPath -Path $backup -Parent (Join-Path $workspace 'temp') -Label 'active backup'
New-Item -ItemType Directory -Path $deployment -Force | Out-Null

$activeMoved = $false
$libraryMoved = $false
try {
    Expand-Archive -LiteralPath $ArchivePath -DestinationPath $deployment
    $wrapper = @(Get-ChildItem -LiteralPath $deployment -Directory)
    if ($wrapper.Count -ne 1) {
        throw 'Verified archive did not extract to exactly one wrapper folder.'
    }
    $next = $wrapper[0].FullName
    & (Join-Path $PSScriptRoot 'verify-portable-build.ps1') `
        -PortableRoot $next `
        -ExpectedVersion $Version

    Copy-PortableUserState -ActiveRoot $active -NextRoot $next

    if (Test-Path -LiteralPath $active) {
        Move-Item -LiteralPath $active -Destination $backup
        $activeMoved = $true
    }
    Move-Item -LiteralPath $next -Destination $active
    if ($activeMoved) {
        $libraryMoved = Move-PortableLibraryState -PreviousRoot $backup -NextRoot $active
    }
    Invoke-PortableHealthCheck -Executable (Join-Path $active 'GameVault.exe')

    if ($activeMoved -and (Test-Path -LiteralPath $backup)) {
        Remove-Item -LiteralPath $backup -Recurse -Force
    }
}
catch {
    if ($libraryMoved -and (Test-Path -LiteralPath (Join-Path $active 'library'))) {
        if (Test-Path -LiteralPath (Join-Path $backup 'library')) {
            throw 'Portable rollback stopped because both builds contain managed library data.'
        }
        Move-Item `
            -LiteralPath (Join-Path $active 'library') `
            -Destination (Join-Path $backup 'library')
        $libraryMoved = $false
    }
    if (Test-Path -LiteralPath $active) {
        Remove-Item -LiteralPath $active -Recurse -Force
    }
    if ($activeMoved -and (Test-Path -LiteralPath $backup)) {
        Move-Item -LiteralPath $backup -Destination $active
    }
    throw
}
finally {
    if (Test-Path -LiteralPath $deployment) {
        Remove-Item -LiteralPath $deployment -Recurse -Force
    }
}

Write-Host 'active-build transaction completed and verified.' -ForegroundColor Green

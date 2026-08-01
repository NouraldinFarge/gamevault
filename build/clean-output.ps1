[CmdletBinding()]
param([Parameter(Mandatory)][string]$WorkspaceRoot)

. (Join-Path $PSScriptRoot 'common.ps1')

$workspace = [System.IO.Path]::GetFullPath($WorkspaceRoot)
foreach ($relative in @('output', 'temp')) {
    $target = [System.IO.Path]::GetFullPath((Join-Path $workspace $relative))
    Assert-ChildPath -Path $target -Parent $workspace -Label $relative
    if (Test-Path -LiteralPath $target) {
        Remove-Item -LiteralPath $target -Recurse -Force
    }
    New-Item -ItemType Directory -Path $target -Force | Out-Null
}
Write-Host 'Generated output cleaned.' -ForegroundColor Green


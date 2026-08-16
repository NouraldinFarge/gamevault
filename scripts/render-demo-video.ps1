[CmdletBinding()]
param(
    [string]$WorkspaceRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
$ffmpeg = (Get-Command ffmpeg -ErrorAction Stop).Source
$mediaRoot = Join-Path $WorkspaceRoot 'docs\media'
$output = Join-Path $mediaRoot 'gamevault-product-tour.mp4'
$poster = Join-Path $mediaRoot 'gamevault-product-tour-poster.jpg'
$font = 'C\:/Windows/Fonts/segoeui.ttf'

$sources = @(
    @{ Path = Join-Path $WorkspaceRoot '.github\social-preview.png'; Duration = 7 },
    @{ Path = Join-Path $WorkspaceRoot 'docs\images\gamevault-home.jpg'; Duration = 8 },
    @{ Path = Join-Path $WorkspaceRoot 'docs\images\gamevault-library.jpg'; Duration = 8 },
    @{ Path = Join-Path $WorkspaceRoot 'docs\images\gamevault-local-files.jpg'; Duration = 8 },
    @{ Path = Join-Path $mediaRoot 'archive-review-plan.jpg'; Duration = 10 },
    @{ Path = Join-Path $WorkspaceRoot '.github\social-preview.png'; Duration = 8 }
)

foreach ($source in $sources) {
    if (-not (Test-Path -LiteralPath $source.Path -PathType Leaf)) {
        throw "Missing demo source: $($source.Path)"
    }
}

New-Item -ItemType Directory -Force -Path $mediaRoot | Out-Null

$captions = @(
    'Portable. Local-first. Review before extraction.',
    'Keep a fast offline view of the games you already own.',
    'Search, filter, favorite, and launch without a cloud dependency.',
    'Inspect ZIP structure and prerequisites before promotion.',
    'Review the title, executable, file plan, and rollback boundary.',
    'Try the synthetic demo - it has no filesystem authority.'
)

$arguments = @('-y')
foreach ($source in $sources) {
    $arguments += @('-loop', '1', '-t', [string]$source.Duration, '-i', $source.Path)
}

$filters = for ($index = 0; $index -lt $sources.Count; $index++) {
    $caption = $captions[$index].Replace("'", "\\'")
    "[$index`:v]scale=1600:1000:force_original_aspect_ratio=decrease," +
        "pad=1600:1000:(ow-iw)/2:(oh-ih)/2:color=0x07101f,setsar=1,fps=30," +
        "drawbox=x=0:y=ih-112:w=iw:h=112:color=0x07101f@0.90:t=fill," +
        "drawtext=fontfile='$font':text='$caption':fontcolor=white:fontsize=36:" +
        "x=(w-text_w)/2:y=h-74,format=yuv420p[v$index]"
}

$filters += '[v0][v1]xfade=transition=fade:duration=0.8:offset=6.2[x1]'
$filters += '[x1][v2]xfade=transition=fade:duration=0.8:offset=13.4[x2]'
$filters += '[x2][v3]xfade=transition=fade:duration=0.8:offset=20.6[x3]'
$filters += '[x3][v4]xfade=transition=fade:duration=0.8:offset=27.8[x4]'
$filters += '[x4][v5]xfade=transition=fade:duration=0.8:offset=37.0[outv]'

$arguments += @(
    '-filter_complex', ($filters -join ';'),
    '-map', '[outv]',
    '-t', '45',
    '-an',
    '-c:v', 'libx264',
    '-preset', 'slow',
    '-crf', '23',
    '-movflags', '+faststart',
    '-pix_fmt', 'yuv420p',
    $output
)

& $ffmpeg @arguments
if ($LASTEXITCODE -ne 0) {
    throw "ffmpeg failed to render the product tour with exit code $LASTEXITCODE."
}

$posterFilter =
    "scale=1600:1000:force_original_aspect_ratio=decrease," +
    "pad=1600:1000:(ow-iw)/2:(oh-ih)/2:color=0x07101f," +
    "drawbox=x=0:y=0:w=iw:h=118:color=0x07101f@0.92:t=fill," +
    "drawtext=fontfile='$font':text='45-second GameVault product tour':" +
    "fontcolor=white:fontsize=42:x=(w-text_w)/2:y=40"

& $ffmpeg -y -i (Join-Path $mediaRoot 'archive-review-plan.jpg') -vf $posterFilter -frames:v 1 -q:v 2 $poster
if ($LASTEXITCODE -ne 0) {
    throw "ffmpeg failed to render the product-tour poster with exit code $LASTEXITCODE."
}

Get-Item -LiteralPath $output, $poster | Select-Object FullName, Length

param(
    [string]$InputDir = "artifacts/readme/scenes",
    [string]$OutputDir = "docs/images"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$bg = [System.Drawing.Color]::FromArgb(17, 24, 39)
$fg = [System.Drawing.Color]::FromArgb(229, 231, 235)
$accent = [System.Drawing.Color]::FromArgb(56, 189, 248)
$font = New-Object System.Drawing.Font("Consolas", 15, [System.Drawing.FontStyle]::Regular)
$titleFont = New-Object System.Drawing.Font("Consolas", 13, [System.Drawing.FontStyle]::Bold)

$files = Get-ChildItem -Path $InputDir -Filter *.txt | Sort-Object Name
foreach ($file in $files) {
    $lines = Get-Content $file.FullName -Encoding UTF8
    if ($lines.Count -eq 0) {
        continue
    }

    $dummyBmp = New-Object System.Drawing.Bitmap(1, 1)
    $gMeasure = [System.Drawing.Graphics]::FromImage($dummyBmp)
    $lineHeight = [Math]::Ceiling($font.GetHeight($gMeasure)) + 2
    $maxWidth = 0
    foreach ($line in $lines) {
        $w = [Math]::Ceiling($gMeasure.MeasureString($line, $font).Width)
        if ($w -gt $maxWidth) {
            $maxWidth = $w
        }
    }
    $gMeasure.Dispose()
    $dummyBmp.Dispose()

    $padding = 24
    $titleBar = 36
    $width = [Math]::Max(1200, $maxWidth + $padding * 2)
    $height = [Math]::Max(700, $titleBar + $padding + ($lineHeight * $lines.Count) + $padding)

    $bmp = New-Object System.Drawing.Bitmap($width, $height)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit
    $gfx.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $gfx.Clear($bg)

    $titleBrush = New-Object System.Drawing.SolidBrush($accent)
    $textBrush = New-Object System.Drawing.SolidBrush($fg)

    $title = "DNA VisiCalc - " + [System.IO.Path]::GetFileNameWithoutExtension($file.Name)
    $gfx.DrawString($title, $titleFont, $titleBrush, 16, 8)

    $y = $titleBar
    foreach ($line in $lines) {
        $gfx.DrawString($line, $font, $textBrush, $padding, $y)
        $y += $lineHeight
    }

    $outName = [System.IO.Path]::GetFileNameWithoutExtension($file.Name) + ".png"
    $outPath = Join-Path $OutputDir $outName
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)

    $titleBrush.Dispose()
    $textBrush.Dispose()
    $gfx.Dispose()
    $bmp.Dispose()
}

$font.Dispose()
$titleFont.Dispose()

Write-Output "Rendered PNG screenshots to $OutputDir"

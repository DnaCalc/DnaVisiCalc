param(
    [string]$InputDir = "artifacts/readme/scenes",
    [string]$OutputDir = "docs/images"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$bgTop = [System.Drawing.Color]::FromArgb(8, 28, 44)
$bgBottom = [System.Drawing.Color]::FromArgb(16, 23, 38)
$fg = [System.Drawing.Color]::FromArgb(226, 232, 240)
$muted = [System.Drawing.Color]::FromArgb(148, 163, 184)
$accent = [System.Drawing.Color]::FromArgb(110, 231, 183)
$status = [System.Drawing.Color]::FromArgb(125, 211, 252)
$ok = [System.Drawing.Color]::FromArgb(167, 243, 208)
$warn = [System.Drawing.Color]::FromArgb(253, 230, 138)
$lineColor = [System.Drawing.Color]::FromArgb(71, 85, 105)

$palette = @(
    [System.Drawing.Color]::FromArgb(186, 230, 253), # sky
    [System.Drawing.Color]::FromArgb(167, 243, 208), # mint
    [System.Drawing.Color]::FromArgb(221, 214, 254), # lavender
    [System.Drawing.Color]::FromArgb(253, 224, 203), # peach
    [System.Drawing.Color]::FromArgb(252, 231, 243), # rose
    [System.Drawing.Color]::FromArgb(254, 240, 138)  # sand
)

$font = New-Object System.Drawing.Font("Consolas", 15, [System.Drawing.FontStyle]::Regular)
$fontBold = New-Object System.Drawing.Font("Consolas", 15, [System.Drawing.FontStyle]::Bold)
$fontItalic = New-Object System.Drawing.Font("Consolas", 15, [System.Drawing.FontStyle]::Italic)
$titleFont = New-Object System.Drawing.Font("Consolas", 14, [System.Drawing.FontStyle]::Bold)

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
    $gfx.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality

    $bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        (New-Object System.Drawing.Rectangle 0, 0, $width, $height),
        $bgTop,
        $bgBottom,
        90
    )
    $gfx.FillRectangle($bgBrush, 0, 0, $width, $height)

    $cardBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(28, 42, 60))
    $cardRect = [System.Drawing.Rectangle]::new(10, 10, ($width - 20), ($height - 20))
    $gfx.FillRectangle($cardBrush, $cardRect)
    $cardPen = New-Object System.Drawing.Pen($lineColor, 2)
    $gfx.DrawRectangle($cardPen, $cardRect)

    $titleBrush = New-Object System.Drawing.SolidBrush($accent)
    $textBrush = New-Object System.Drawing.SolidBrush($fg)
    $mutedBrush = New-Object System.Drawing.SolidBrush($muted)
    $statusBrush = New-Object System.Drawing.SolidBrush($status)
    $okBrush = New-Object System.Drawing.SolidBrush($ok)
    $warnBrush = New-Object System.Drawing.SolidBrush($warn)

    $title = "DNA VisiCalc - " + [System.IO.Path]::GetFileNameWithoutExtension($file.Name)
    $gfx.DrawString($title, $titleFont, $titleBrush, 16, 8)

    $y = $titleBar
    foreach ($line in $lines) {
        $lineBrush = $textBrush
        $lineFont = $font

        if ($line -match "^Status:") {
            $lineBrush = $statusBrush
            if ($line -match "Ready|Set|saved|open") {
                $lineBrush = $okBrush
            }
            if ($line -match "error|invalid|failed") {
                $lineBrush = $warnBrush
            }
        } elseif ($line -match "^Mode:|^\|Workbook|^\|File:") {
            $lineBrush = $statusBrush
        } elseif ($line -match "^DNA VisiCalc|^\|DNA VisiCalc") {
            $lineBrush = $titleBrush
            $lineFont = $fontBold
        } elseif ($line -match "fmt |LET|LAMBDA|MAP|INDIRECT|OFFSET|SEQUENCE|RANDARRAY") {
            $lineBrush = $warnBrush
        } elseif ($line -match "^\|[- ]{5,}$|^[ -]{8,}$") {
            $lineBrush = $mutedBrush
        }

        if ($line -match "\|") {
            $tokens = [regex]::Split($line, "(\|)")
            $x = $padding
            $columnIndex = 0
            foreach ($token in $tokens) {
                if ($token -eq "") {
                    continue
                }

                $tokenBrush = $lineBrush
                $tokenFont = $lineFont
                if ($token -eq "|") {
                    $tokenBrush = $mutedBrush
                } elseif ($line -match "^\s*\d+\s*\|") {
                    $tokenBrush = New-Object System.Drawing.SolidBrush($palette[$columnIndex % $palette.Count])
                    $tokenFont = if ($columnIndex -eq 0) { $fontBold } else { $font }
                    $columnIndex++
                } elseif ($line -match "^\s*\|\s*[A-Z]\s*\|") {
                    $tokenBrush = New-Object System.Drawing.SolidBrush($palette[$columnIndex % $palette.Count])
                    $tokenFont = $fontBold
                    $columnIndex++
                }

                $gfx.DrawString($token, $tokenFont, $tokenBrush, $x, $y)
                $x += [Math]::Ceiling($gfx.MeasureString($token, $tokenFont).Width)
                if ($tokenBrush -is [System.Drawing.SolidBrush] -and $tokenBrush -ne $lineBrush -and $tokenBrush -ne $mutedBrush -and $tokenBrush -ne $textBrush -and $tokenBrush -ne $titleBrush -and $tokenBrush -ne $statusBrush -and $tokenBrush -ne $okBrush -and $tokenBrush -ne $warnBrush) {
                    $tokenBrush.Dispose()
                }
            }
        } else {
            $gfx.DrawString($line, $lineFont, $lineBrush, $padding, $y)
        }
        $y += $lineHeight
    }

    $outName = [System.IO.Path]::GetFileNameWithoutExtension($file.Name) + ".png"
    $outPath = Join-Path $OutputDir $outName
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)

    $bgBrush.Dispose()
    $cardBrush.Dispose()
    $cardPen.Dispose()
    $titleBrush.Dispose()
    $textBrush.Dispose()
    $mutedBrush.Dispose()
    $statusBrush.Dispose()
    $okBrush.Dispose()
    $warnBrush.Dispose()
    $gfx.Dispose()
    $bmp.Dispose()
}

$font.Dispose()
$fontBold.Dispose()
$fontItalic.Dispose()
$titleFont.Dispose()

Write-Output "Rendered PNG screenshots to $OutputDir"

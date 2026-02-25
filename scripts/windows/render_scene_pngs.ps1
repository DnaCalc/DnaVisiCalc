param(
    [string]$InputDir = "artifacts/readme/scenes",
    [string]$OutputDir = "docs/images"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# ---------------------------------------------------------------------------
# Chrome colours (gradient background, card border, title bar)
# ---------------------------------------------------------------------------
$bgTop       = [System.Drawing.Color]::FromArgb(8, 28, 44)
$bgBottom    = [System.Drawing.Color]::FromArgb(16, 23, 38)
$cardColor   = [System.Drawing.Color]::FromArgb(28, 42, 60)
$lineColor   = [System.Drawing.Color]::FromArgb(71, 85, 105)
$accentColor = [System.Drawing.Color]::FromArgb(110, 231, 183)

# Default text colour when span fg is null (terminal default)
$defaultFg   = [System.Drawing.Color]::FromArgb(226, 232, 240)

# ---------------------------------------------------------------------------
# Fonts
# ---------------------------------------------------------------------------
$fontRegular    = New-Object System.Drawing.Font("Consolas", 15, [System.Drawing.FontStyle]::Regular)
$fontBold       = New-Object System.Drawing.Font("Consolas", 15, [System.Drawing.FontStyle]::Bold)
$fontItalic     = New-Object System.Drawing.Font("Consolas", 15, [System.Drawing.FontStyle]::Italic)
$fontBoldItalic = New-Object System.Drawing.Font("Consolas", 15, ([System.Drawing.FontStyle]::Bold -bor [System.Drawing.FontStyle]::Italic))
$titleFont      = New-Object System.Drawing.Font("Consolas", 14, [System.Drawing.FontStyle]::Bold)

# ---------------------------------------------------------------------------
# StringFormat: GenericTypographic removes the internal padding that causes
# column drift when GDI+ DrawString renders multi-character strings.
# ---------------------------------------------------------------------------
$sf = [System.Drawing.StringFormat]::GenericTypographic.Clone()
$sf.FormatFlags = $sf.FormatFlags -bor [System.Drawing.StringFormatFlags]::MeasureTrailingSpaces

# ---------------------------------------------------------------------------
# Measure a single Consolas character width for pixel-perfect monospace grid.
# Use GenericTypographic so the measurement matches the drawing format.
# ---------------------------------------------------------------------------
$dummyBmp  = New-Object System.Drawing.Bitmap(1, 1)
$gMeasure  = [System.Drawing.Graphics]::FromImage($dummyBmp)
$gMeasure.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit

$sampleLen  = 80
$sampleText = "M" * $sampleLen
$sampleSize = $gMeasure.MeasureString($sampleText, $fontRegular, [int]::MaxValue, $sf)
$charWidth  = $sampleSize.Width / $sampleLen
$lineHeight = [Math]::Ceiling($fontRegular.GetHeight($gMeasure)) + 2

$gMeasure.Dispose()
$dummyBmp.Dispose()

# ---------------------------------------------------------------------------
# Helper: parse #RRGGBB hex string to Drawing.Color
# ---------------------------------------------------------------------------
function HexToColor([string]$hex) {
    if (-not $hex -or $hex -eq "null") { return $null }
    $hex = $hex.TrimStart("#")
    $r = [Convert]::ToInt32($hex.Substring(0, 2), 16)
    $g = [Convert]::ToInt32($hex.Substring(2, 2), 16)
    $b = [Convert]::ToInt32($hex.Substring(4, 2), 16)
    return [System.Drawing.Color]::FromArgb($r, $g, $b)
}

# ---------------------------------------------------------------------------
# Helper: pick font variant from bold/italic flags
# ---------------------------------------------------------------------------
function PickFont([bool]$bold, [bool]$italic) {
    if ($bold -and $italic) { return $fontBoldItalic }
    if ($bold)              { return $fontBold }
    if ($italic)            { return $fontItalic }
    return $fontRegular
}

# ---------------------------------------------------------------------------
# Helper: ensure fg has enough contrast against bg.
# Uses perceived-brightness difference; when too low, darkens fg on light
# backgrounds and lightens fg on dark backgrounds.
# ---------------------------------------------------------------------------
function EnsureContrast($fgColor, $bgColor) {
    $fgBright = (0.299 * $fgColor.R + 0.587 * $fgColor.G + 0.114 * $fgColor.B) / 255.0
    $bgBright = (0.299 * $bgColor.R + 0.587 * $bgColor.G + 0.114 * $bgColor.B) / 255.0

    $diff = [Math]::Abs($fgBright - $bgBright)
    $minDiff = 0.30
    if ($diff -ge $minDiff) { return $fgColor }

    # Graduated shift: the smaller the current diff, the stronger the push
    $need = $minDiff - $diff

    if ($bgBright -gt 0.5) {
        # Light background -> darken fg
        $t = [Math]::Min(0.55, $need * 2.5)
        $r = [int]($fgColor.R * (1.0 - $t))
        $g = [int]($fgColor.G * (1.0 - $t))
        $b = [int]($fgColor.B * (1.0 - $t))
    } else {
        # Dark background -> lighten fg
        $t = [Math]::Min(0.55, $need * 2.5)
        $r = [Math]::Min(255, [int]($fgColor.R + (255 - $fgColor.R) * $t))
        $g = [Math]::Min(255, [int]($fgColor.G + (255 - $fgColor.G) * $t))
        $b = [Math]::Min(255, [int]($fgColor.B + (255 - $fgColor.B) * $t))
    }
    return [System.Drawing.Color]::FromArgb($r, $g, $b)
}

# ---------------------------------------------------------------------------
# Render each .json scene file
# ---------------------------------------------------------------------------
$files = Get-ChildItem -Path $InputDir -Filter *.json | Sort-Object Name
foreach ($file in $files) {
    $json = Get-Content $file.FullName -Raw -Encoding UTF8 | ConvertFrom-Json

    $gridWidth  = [int]$json.width
    $gridHeight = [int]$json.height

    $padding  = 24
    $titleBar = 36
    $imgWidth  = [Math]::Max(1200, [int][Math]::Ceiling($padding + $gridWidth * $charWidth + $padding))
    $imgHeight = [Math]::Max(700, $titleBar + $padding + [int]($lineHeight * $gridHeight) + $padding)

    $bmp = New-Object System.Drawing.Bitmap($imgWidth, $imgHeight)
    $gfx = [System.Drawing.Graphics]::FromImage($bmp)
    $gfx.TextRenderingHint  = [System.Drawing.Text.TextRenderingHint]::ClearTypeGridFit
    $gfx.SmoothingMode      = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $gfx.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality

    # -- Gradient background --
    $bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        (New-Object System.Drawing.Rectangle 0, 0, $imgWidth, $imgHeight),
        $bgTop, $bgBottom, 90
    )
    $gfx.FillRectangle($bgBrush, 0, 0, $imgWidth, $imgHeight)

    # -- Card with border --
    $cardBrush = New-Object System.Drawing.SolidBrush($cardColor)
    $cardRect  = [System.Drawing.Rectangle]::new(10, 10, ($imgWidth - 20), ($imgHeight - 20))
    $gfx.FillRectangle($cardBrush, $cardRect)
    $cardPen = New-Object System.Drawing.Pen($lineColor, 2)
    $gfx.DrawRectangle($cardPen, $cardRect)

    # -- Title bar --
    $titleBrush = New-Object System.Drawing.SolidBrush($accentColor)
    $title = "DNA VisiCalc - " + [System.IO.Path]::GetFileNameWithoutExtension($file.Name)
    $gfx.DrawString($title, $titleFont, $titleBrush, 16, 8, $sf)

    # -- Render rows from JSON spans --
    # Each character is drawn individually at its exact grid position to
    # avoid cumulative drift from GDI+ inter-character spacing.
    foreach ($row in $json.rows) {
        $y = $titleBar + [int]$row.y * $lineHeight
        $charIdx = 0

        foreach ($span in $row.spans) {
            $text = [string]$span.text
            $spanLen = $text.Length

            # --- Foreground colour ---
            $fgColor = $defaultFg
            if ($span.fg -and $span.fg -ne "null") {
                $parsed = HexToColor $span.fg
                if ($parsed) { $fgColor = $parsed }
            }

            # --- Background colour ---
            $bgColorVal = $null
            if ($span.bg -and $span.bg -ne "null") {
                $bgColorVal = HexToColor $span.bg
            }

            # --- Font variant ---
            $spanBold   = if ($span.bold   -is [bool]) { $span.bold }   else { $false }
            $spanItalic = if ($span.italic -is [bool]) { $span.italic } else { $false }
            $spanFont   = PickFont $spanBold $spanItalic

            # --- Background rectangle (per-span for efficiency) ---
            if ($bgColorVal) {
                $bx  = $padding + [Math]::Floor($charIdx * $charWidth)
                $bgBr = New-Object System.Drawing.SolidBrush($bgColorVal)
                $bgW  = [Math]::Ceiling($spanLen * $charWidth)
                $gfx.FillRectangle($bgBr, [int]$bx, [int]$y, [int]$bgW, [int]$lineHeight)
                $bgBr.Dispose()
            }

            # --- Contrast heuristic ---
            $effectiveBg = if ($bgColorVal) { $bgColorVal } else { $cardColor }
            $fgColor = EnsureContrast $fgColor $effectiveBg

            # --- Draw text char-by-char at exact grid positions ---
            $fgBrush = New-Object System.Drawing.SolidBrush($fgColor)
            for ($i = 0; $i -lt $spanLen; $i++) {
                $ch = $text[$i]
                if ($ch -ne ' ') {
                    $cx = $padding + [Math]::Floor(($charIdx + $i) * $charWidth)
                    $gfx.DrawString($ch.ToString(), $spanFont, $fgBrush, [float]$cx, [float]$y, $sf)
                }
            }
            $fgBrush.Dispose()

            $charIdx += $spanLen
        }
    }

    # -- Save PNG --
    $outName = [System.IO.Path]::GetFileNameWithoutExtension($file.Name) + ".png"
    $outPath = Join-Path $OutputDir $outName
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)

    # -- Dispose --
    $bgBrush.Dispose()
    $cardBrush.Dispose()
    $cardPen.Dispose()
    $titleBrush.Dispose()
    $gfx.Dispose()
    $bmp.Dispose()
}

$fontRegular.Dispose()
$fontBold.Dispose()
$fontItalic.Dispose()
$fontBoldItalic.Dispose()
$titleFont.Dispose()
$sf.Dispose()

Write-Output "Rendered $($files.Count) PNG screenshots to $OutputDir"

#!/usr/bin/env sh
set -eu

cd "$(dirname "$0")"

# ImageMagick v7 prefers `magick`; fall back to `convert`.
if command -v magick >/dev/null 2>&1; then
  IM='magick'
else
  IM='convert'
fi

# ImageMagick 7: input file before -resize
# Create template icons for the menu bar
$IM glyph.svg -background none -resize 22x22 iconTemplate.png
$IM glyph.svg -background none -resize 44x44 'iconTemplate@2x.png'
$IM glyph.svg -background none -resize 22x22 iconTemplateUpdate.png
$IM glyph.svg -background none -resize 44x44 'iconTemplateUpdate@2x.png'

# Create main application icons from icon.svg
$IM icon.svg -background none -resize 1024x1024 icon.png
$IM icon.svg -background none -resize 2048x2048 'icon@2x.png'
$IM icon.svg -background none -resize 512x512 icon-512.png

# Light / dark-theme variants
$IM icon-light.svg -background none -resize 1024x1024 icon-light.png

# Create Windows icon (ico) with multiple sizes
$IM icon.svg -background none -define icon:auto-resize=256,128,64,48,32,16 icon.ico

# Create macOS icon set (icns)
mkdir -p icon.iconset
$IM icon.svg -background none -resize 16x16 icon.iconset/icon_16x16.png
$IM icon.svg -background none -resize 32x32 'icon.iconset/icon_16x16@2x.png'
$IM icon.svg -background none -resize 32x32 icon.iconset/icon_32x32.png
$IM icon.svg -background none -resize 64x64 'icon.iconset/icon_32x32@2x.png'
$IM icon.svg -background none -resize 128x128 icon.iconset/icon_128x128.png
$IM icon.svg -background none -resize 256x256 'icon.iconset/icon_128x128@2x.png'
$IM icon.svg -background none -resize 256x256 icon.iconset/icon_256x256.png
$IM icon.svg -background none -resize 512x512 'icon.iconset/icon_256x256@2x.png'
$IM icon.svg -background none -resize 512x512 icon.iconset/icon_512x512.png
$IM icon.svg -background none -resize 1024x1024 'icon.iconset/icon_512x512@2x.png'
iconutil -c icns icon.iconset
rm -rf icon.iconset

# Light icns
mkdir -p icon-light.iconset
$IM icon-light.svg -background none -resize 16x16 icon-light.iconset/icon_16x16.png
$IM icon-light.svg -background none -resize 32x32 'icon-light.iconset/icon_16x16@2x.png'
$IM icon-light.svg -background none -resize 32x32 icon-light.iconset/icon_32x32.png
$IM icon-light.svg -background none -resize 64x64 'icon-light.iconset/icon_32x32@2x.png'
$IM icon-light.svg -background none -resize 128x128 icon-light.iconset/icon_128x128.png
$IM icon-light.svg -background none -resize 256x256 'icon-light.iconset/icon_128x128@2x.png'
$IM icon-light.svg -background none -resize 256x256 icon-light.iconset/icon_256x256.png
$IM icon-light.svg -background none -resize 512x512 'icon-light.iconset/icon_256x256@2x.png'
$IM icon-light.svg -background none -resize 512x512 icon-light.iconset/icon_512x512.png
$IM icon-light.svg -background none -resize 1024x1024 'icon-light.iconset/icon_512x512@2x.png'
iconutil -c icns icon-light.iconset -o icon-light.icns
rm -rf icon-light.iconset

echo "Generated Avocado Work icons from landing mark"

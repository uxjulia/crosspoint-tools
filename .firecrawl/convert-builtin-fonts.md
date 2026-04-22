#!/bin/bash

set -e

cd "$(dirname "$0")"

READER\_FONT\_STYLES=("Regular" "Italic" "Bold" "BoldItalic")
NOTOSERIF\_FONT\_SIZES=(12 14 16 18)
NOTOSANS\_FONT\_SIZES=(12 14 16 18)
OPENDYSLEXIC\_FONT\_SIZES=(8 10 12 14)

for size in ${NOTOSERIF\_FONT\_SIZES\[@\]}; do
 for style in ${READER\_FONT\_STYLES\[@\]}; do
 font\_name="notoserif\_${size}\_$(echo $style \| tr '\[:upper:\]' '\[:lower:\]')"
 font\_path="../builtinFonts/source/NotoSerif/NotoSerif-${style}.ttf"
 output\_path="../builtinFonts/${font\_name}.h"
 python fontconvert.py $font\_name $size $font\_path --2bit --compress --pnum > $output\_path
 echo "Generated $output\_path"
 done
done

for size in ${NOTOSANS\_FONT\_SIZES\[@\]}; do
 for style in ${READER\_FONT\_STYLES\[@\]}; do
 font\_name="notosans\_${size}\_$(echo $style \| tr '\[:upper:\]' '\[:lower:\]')"
 font\_path="../builtinFonts/source/NotoSans/NotoSans-${style}.ttf"
 output\_path="../builtinFonts/${font\_name}.h"
 python fontconvert.py $font\_name $size $font\_path --2bit --compress --pnum > $output\_path
 echo "Generated $output\_path"
 done
done

for size in ${OPENDYSLEXIC\_FONT\_SIZES\[@\]}; do
 for style in ${READER\_FONT\_STYLES\[@\]}; do
 font\_name="opendyslexic\_${size}\_$(echo $style \| tr '\[:upper:\]' '\[:lower:\]')"
 font\_path="../builtinFonts/source/OpenDyslexic/OpenDyslexic-${style}.otf"
 output\_path="../builtinFonts/${font\_name}.h"
 python fontconvert.py $font\_name $size $font\_path --2bit --compress > $output\_path
 echo "Generated $output\_path"
 done
done

UI\_FONT\_SIZES=(10 12)
UI\_FONT\_STYLES=("Regular" "Bold")

for size in ${UI\_FONT\_SIZES\[@\]}; do
 for style in ${UI\_FONT\_STYLES\[@\]}; do
 font\_name="ubuntu\_${size}\_$(echo $style \| tr '\[:upper:\]' '\[:lower:\]')"
 font\_path="../builtinFonts/source/Ubuntu/Ubuntu-${style}.ttf"
 output\_path="../builtinFonts/${font\_name}.h"
 python fontconvert.py $font\_name $size $font\_path > $output\_path
 echo "Generated $output\_path"
 done
done

python fontconvert.py notosans\_8\_regular 8 ../builtinFonts/source/NotoSans/NotoSans-Regular.ttf > ../builtinFonts/notosans\_8\_regular.h

echo ""
echo "Running compression verification..."
python verify\_compression.py ../builtinFonts/
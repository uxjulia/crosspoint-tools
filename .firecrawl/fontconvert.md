#!python3
import freetype
import zlib
import sys
import re
import math
import argparse
from collections import namedtuple
from fontTools.ttLib import TTFont

\# Originally from https://github.com/vroland/epdiy

parser = argparse.ArgumentParser(description="Generate a header file from a font to be used with epdiy.")
parser.add\_argument("name", action="store", help="name of the font.")
parser.add\_argument("size", type=int, help="font size to use.")
parser.add\_argument("fontstack", action="store", nargs='+', help="list of font files, ordered by descending priority.")
parser.add\_argument("--2bit", dest="is2Bit", action="store\_true", help="generate 2-bit greyscale bitmap instead of 1-bit black and white.")
parser.add\_argument("--additional-intervals", dest="additional\_intervals", action="append", help="Additional code point intervals to export as min,max. This argument can be repeated.")
parser.add\_argument("--compress", dest="compress", action="store\_true", help="Compress glyph bitmaps using DEFLATE with group-based compression.")
parser.add\_argument("--force-autohint", dest="force\_autohint", action="store\_true", help="Force FreeType auto-hinter instead of native font hinting. Improves stem width consistency for fonts with weak or no native TrueType hints.")
parser.add\_argument("--pnum", dest="pnum", action="store\_true", help="Use proportional numerals (pnum OpenType feature) instead of default tabular figures. Reduces visual gaps between digits in running prose.")
args = parser.parse\_args()

GlyphProps = namedtuple("GlyphProps", \["width", "height", "advance\_x", "left", "top", "data\_length", "data\_offset", "code\_point"\])

font\_stack = \[freetype.Face(f) for f in args.fontstack\]
is2Bit = args.is2Bit
size = args.size
font\_name = args.name
load\_flags = freetype.FT\_LOAD\_RENDER
if args.force\_autohint:
 load\_flags \|= freetype.FT\_LOAD\_FORCE\_AUTOHINT

\# inclusive unicode code point intervals
\# must not overlap and be in ascending order
intervals = \[\
 ### Basic Latin ###\
 # ASCII letters, digits, punctuation, control characters\
 (0x0000, 0x007F),\
 ### Latin-1 Supplement ###\
 # Accented characters for Western European languages\
 (0x0080, 0x00FF),\
 ### Latin Extended-A ###\
 # Eastern European and Baltic languages\
 (0x0100, 0x017F),\
 ### Latin Extended-B (Vietnamese subset only) ###\
 # Only Ơ/ơ (U+01A0-01A1), Ư/ư (U+01AF-01B0) for Vietnamese\
 (0x01A0, 0x01A1),\
 (0x01AF, 0x01B0),\
 ### Latin Extended-B (European subset only) ###\
 # Croatian digraphs (DŽ/Lj/Nj), Pinyin caron variants,\
 # European diacritical variants, Romanian (Ș/ș/Ț/ț)\
 (0x01C4, 0x021F),\
 ### Vietnamese Extended ###\
 # All precomposed Vietnamese characters with tone marks\
 # Ả Ấ Ầ Ẩ Ẫ Ậ Ắ Ằ Ẳ Ẵ Ặ Ẹ Ẻ Ẽ Ế Ề Ể Ễ Ệ Ỉ Ị Ọ Ỏ Ố Ồ Ổ Ỗ Ộ Ớ Ờ Ở Ỡ Ợ Ụ Ủ Ứ Ừ Ử Ữ Ự Ỳ Ỵ Ỷ Ỹ\
 (0x1EA0, 0x1EF9),\
 ### General Punctuation (core subset) ###\
 # Smart quotes, en dash, em dash, ellipsis, NO-BREAK SPACE\
 (0x2000, 0x206F),\
 ### Basic Symbols From "Latin-1 + Misc" ###\
 # dashes, quotes, prime marks\
 (0x2010, 0x203A),\
 # misc punctuation\
 (0x2040, 0x205F),\
 # common currency symbols\
 (0x20A0, 0x20CF),\
 ### Combining Diacritical Marks (minimal subset) ###\
 # Needed for proper rendering of many extended Latin languages\
 (0x0300, 0x036F),\
 ### Greek & Coptic ###\
 # Used in science, maths, philosophy, some academic texts\
 # (0x0370, 0x03FF),\
 ### Cyrillic ###\
 # Russian, Ukrainian, Bulgarian, etc.\
 (0x0400, 0x04FF),\
 ### Math Symbols (common subset) ###\
 # Superscripts and Subscripts\
 (0x2070, 0x209F),\
 # General math operators\
 (0x2200, 0x22FF),\
 # Arrows\
 (0x2190, 0x21FF),\
 ### CJK ###\
 # Core Unified Ideographs\
 # (0x4E00, 0x9FFF),\
 # # Extension A\
 # (0x3400, 0x4DBF),\
 # # Extension B\
 # (0x20000, 0x2A6DF),\
 # # Extension C–F\
 # (0x2A700, 0x2EBEF),\
 # # Extension G\
 # (0x30000, 0x3134F),\
 # # Hiragana\
 # (0x3040, 0x309F),\
 # # Katakana\
 # (0x30A0, 0x30FF),\
 # # Katakana Phonetic Extensions\
 # (0x31F0, 0x31FF),\
 # # Halfwidth Katakana\
 # (0xFF60, 0xFF9F),\
 # # Hangul Syllables\
 # (0xAC00, 0xD7AF),\
 # # Hangul Jamo\
 # (0x1100, 0x11FF),\
 # # Hangul Compatibility Jamo\
 # (0x3130, 0x318F),\
 # # Hangul Jamo Extended-A\
 # (0xA960, 0xA97F),\
 # # Hangul Jamo Extended-B\
 # (0xD7B0, 0xD7FF),\
 # # CJK Radicals Supplement\
 # (0x2E80, 0x2EFF),\
 # # Kangxi Radicals\
 # (0x2F00, 0x2FDF),\
 # # CJK Symbols and Punctuation\
 # (0x3000, 0x303F),\
 # # CJK Compatibility Forms\
 # (0xFE30, 0xFE4F),\
 # # CJK Compatibility Ideographs\
 # (0xF900, 0xFAFF),\
 ### Alphabetic Presentation Forms (Latin ligatures) ###\
 # ff, fi, fl, ffi, ffl, long-st, st\
 (0xFB00, 0xFB06),\
 ### Specials\
 # Replacement Character\
 (0xFFFD, 0xFFFD),\
\]

add\_ints = \[\]
if args.additional\_intervals:
 add\_ints = \[tuple(\[int(n, base=0) for n in i.split(",")\]) for i in args.additional\_intervals\]

def norm\_floor(val):
 return int(math.floor(val / (1 << 6)))

def norm\_ceil(val):
 return int(math.ceil(val / (1 << 6)))

\# Fixed-point (fp4) output conventions (must match EpdFontData.h / fp4 namespace):
#
\# advanceX 12.4 unsigned fixed-point (uint16\_t).
\# 12 integer bits, 4 fractional bits = 1/16-pixel resolution.
\# Encoded from FreeType's 16.16 linearHoriAdvance.
#
\# kernMatrix 4.4 signed fixed-point (int8\_t).
\# 4 integer bits, 4 fractional bits = 1/16-pixel resolution.
\# Range: -8.0 to +7.9375 pixels.
\# Encoded from font design-unit kerning values.
#
\# Both share 4 fractional bits so the renderer can add them directly into a
\# single int32\_t accumulator and defer rounding until pixel placement.

def fp4\_from\_ft16\_16(val):
 """Convert FreeType 16.16 fixed-point to 12.4 fixed-point with rounding."""
 return (val + (1 << 11)) >> 12

def fp4\_from\_design\_units(du, scale):
 """Convert a font design-unit value to 4.4 fixed-point, clamped to int8\_t.

 Multiplies by scale (ppem / units\_per\_em) and shifts into 4 fractional
 bits. The result is rounded to nearest and clamped to \[-128, 127\].
 """
 raw = round(du \* scale \* 16)
 return max(-128, min(127, raw))

def chunks(l, n):
 for i in range(0, len(l), n):
 yield l\[i:i + n\]

def extract\_pnum\_subs(font\_path):
 """Extract pnum (proportional figures) GSUB substitutions.

 Parses the font's GSUB table for the 'pnum' feature, which replaces
 tabular-width figure glyphs with proportional-width alternates.
 Returns {original\_glyph\_name: substitute\_glyph\_name} or empty dict.
 """
 font = TTFont(font\_path)
 subs = {}
 if 'GSUB' not in font:
 font.close()
 return subs
 gsub = font\['GSUB'\].table
 pnum\_indices = set()
 if gsub.FeatureList:
 for fr in gsub.FeatureList.FeatureRecord:
 if fr.FeatureTag == 'pnum':
 pnum\_indices.update(fr.Feature.LookupListIndex)
 for li in pnum\_indices:
 lookup = gsub.LookupList.Lookup\[li\]
 for st in lookup.SubTable:
 actual = st
 if lookup.LookupType == 7 and hasattr(st, 'ExtSubTable'):
 actual = st.ExtSubTable
 if hasattr(actual, 'mapping'):
 subs.update(actual.mapping)
 font.close()
 return subs

\# Build proportional numeral glyph overrides when --pnum is active.
\# Maps (face\_index, codepoint) -> freetype glyph index for the proportional alternate.
pnum\_glyph\_overrides = {}
pnum\_kern\_subs = {} # face\_index -> {original\_glyph\_name: substitute\_glyph\_name}
if args.pnum:
 for face\_idx, font\_path in enumerate(args.fontstack):
 subs = extract\_pnum\_subs(font\_path)
 if not subs:
 continue
 pnum\_kern\_subs\[face\_idx\] = subs
 tt\_font = TTFont(font\_path)
 cmap = tt\_font.getBestCmap() or {}
 glyph\_order = tt\_font.getGlyphOrder()
 name\_to\_glyph\_idx = {name: idx for idx, name in enumerate(glyph\_order)}
 count = 0
 for cp, glyph\_name in cmap.items():
 if glyph\_name in subs:
 sub\_name = subs\[glyph\_name\]
 sub\_idx = name\_to\_glyph\_idx.get(sub\_name, 0)
 if sub\_idx > 0:
 pnum\_glyph\_overrides\[(face\_idx, cp)\] = sub\_idx
 count += 1
 tt\_font.close()
 if count > 0:
 print(f"pnum: {count} glyph substitutions from {font\_path}", file=sys.stderr)

def load\_glyph(code\_point):
 face\_index = 0
 while face\_index < len(font\_stack):
 face = font\_stack\[face\_index\]
 glyph\_index = pnum\_glyph\_overrides.get((face\_index, code\_point))
 if glyph\_index is None:
 glyph\_index = face.get\_char\_index(code\_point)
 if glyph\_index > 0:
 face.load\_glyph(glyph\_index, load\_flags)
 return face
 face\_index += 1
 return None

unmerged\_intervals = sorted(intervals + add\_ints)
intervals = \[\]
unvalidated\_intervals = \[\]
for i\_start, i\_end in unmerged\_intervals:
 if len(unvalidated\_intervals) > 0 and i\_start + 1 <= unvalidated\_intervals\[-1\]\[1\]:
 unvalidated\_intervals\[-1\] = (unvalidated\_intervals\[-1\]\[0\], max(unvalidated\_intervals\[-1\]\[1\], i\_end))
 continue
 unvalidated\_intervals.append((i\_start, i\_end))

for i\_start, i\_end in unvalidated\_intervals:
 start = i\_start
 for code\_point in range(i\_start, i\_end + 1):
 face = load\_glyph(code\_point)
 if face is None:
 if start < code\_point:
 intervals.append((start, code\_point - 1))
 start = code\_point + 1
 if start != i\_end + 1:
 intervals.append((start, i\_end))

for face in font\_stack:
 face.set\_char\_size(size << 6, size << 6, 150, 150)

total\_size = 0
all\_glyphs = \[\]

for i\_start, i\_end in intervals:
 for code\_point in range(i\_start, i\_end + 1):
 face = load\_glyph(code\_point)
 bitmap = face.glyph.bitmap

 # Build out 4-bit greyscale bitmap
 pixels4g = \[\]
 px = 0
 for i, v in enumerate(bitmap.buffer):
 y = i / bitmap.width
 x = i % bitmap.width
 if x % 2 == 0:
 px = (v >> 4)
 else:
 px = px \| (v & 0xF0)
 pixels4g.append(px);
 px = 0
 # eol
 if x == bitmap.width - 1 and bitmap.width % 2 > 0:
 pixels4g.append(px)
 px = 0

 if is2Bit:
 # 0-3 white, 4-7 light grey, 8-11 dark grey, 12-15 black
 # Downsample to 2-bit bitmap
 pixels2b = \[\]
 px = 0
 pitch = (bitmap.width // 2) + (bitmap.width % 2)
 for y in range(bitmap.rows):
 for x in range(bitmap.width):
 px = px << 2
 bm = pixels4g\[y \* pitch + (x // 2)\]
 bm = (bm >> ((x % 2) \* 4)) & 0xF

 if bm >= 12:
 px += 3
 elif bm >= 8:
 px += 2
 elif bm >= 4:
 px += 1

 if (y \* bitmap.width + x) % 4 == 3:
 pixels2b.append(px)
 px = 0
 if (bitmap.width \* bitmap.rows) % 4 != 0:
 px = px << (4 - (bitmap.width \* bitmap.rows) % 4) \* 2
 pixels2b.append(px)

 # for y in range(bitmap.rows):
 # line = ''
 # for x in range(bitmap.width):
 # pixelPosition = y \* bitmap.width + x
 # byte = pixels2b\[pixelPosition // 4\]
 # bit\_index = (3 - (pixelPosition % 4)) \* 2
 # line += '#' if ((byte >> bit\_index) & 3) > 0 else '.'
 # print(line)
 # print('')
 else:
 # Downsample to 1-bit bitmap - treat any 2+ as black
 pixelsbw = \[\]
 px = 0
 pitch = (bitmap.width // 2) + (bitmap.width % 2)
 for y in range(bitmap.rows):
 for x in range(bitmap.width):
 px = px << 1
 bm = pixels4g\[y \* pitch + (x // 2)\]
 px += 1 if ((x & 1) == 0 and bm & 0xE > 0) or ((x & 1) == 1 and bm & 0xE0 > 0) else 0

 if (y \* bitmap.width + x) % 8 == 7:
 pixelsbw.append(px)
 px = 0
 if (bitmap.width \* bitmap.rows) % 8 != 0:
 px = px << (8 - (bitmap.width \* bitmap.rows) % 8)
 pixelsbw.append(px)

 # for y in range(bitmap.rows):
 # line = ''
 # for x in range(bitmap.width):
 # pixelPosition = y \* bitmap.width + x
 # byte = pixelsbw\[pixelPosition // 8\]
 # bit\_index = 7 - (pixelPosition % 8)
 # line += '#' if (byte >> bit\_index) & 1 else '.'
 # print(line)
 # print('')

 pixels = pixels2b if is2Bit else pixelsbw

 # Build output data
 packed = bytes(pixels)
 glyph = GlyphProps(
 width = bitmap.width,
 height = bitmap.rows,
 # We use linearHoriAdvance (16.16 fixed-point, unhinted) instead of
 # advance.x (26.6 fixed-point, grid-fitted to whole pixels by hinter)
 advance\_x = fp4\_from\_ft16\_16(face.glyph.linearHoriAdvance),
 left = face.glyph.bitmap\_left,
 top = face.glyph.bitmap\_top,
 data\_length = len(packed),
 data\_offset = total\_size,
 code\_point = code\_point,
 )
 total\_size += len(packed)
 all\_glyphs.append((glyph, packed))

\# pipe seems to be a good heuristic for the "real" descender
face = load\_glyph(ord('\|'))

glyph\_data = \[\]
glyph\_props = \[\]
for index, glyph in enumerate(all\_glyphs):
 props, packed = glyph
 glyph\_data.extend(\[b for b in packed\])
 glyph\_props.append(props)

\# --- Kerning pair extraction ---
\# Modern fonts store kerning in the OpenType GPOS table, which FreeType's
\# get\_kerning() does not read. We use fonttools to parse both the legacy
\# kern table and the GPOS 'kern' feature (PairPos lookups, including
\# Extension wrappers).

COMBINING\_MARKS\_START = 0x0300
COMBINING\_MARKS\_END = 0x036F
all\_codepoints = \[g.code\_point for g in glyph\_props\]
kernable\_codepoints = set(cp for cp in all\_codepoints
 if not (COMBINING\_MARKS\_START <= cp <= COMBINING\_MARKS\_END))

\# Map each kernable codepoint to the font-stack index that serves it
\# (same priority logic as load\_glyph).
cp\_to\_face\_idx = {}
for cp in kernable\_codepoints:
 for face\_idx, f in enumerate(font\_stack):
 if f.get\_char\_index(cp) > 0:
 cp\_to\_face\_idx\[cp\] = face\_idx
 break

\# Group codepoints by face index
face\_idx\_cps = {}
for cp, fi in cp\_to\_face\_idx.items():
 face\_idx\_cps.setdefault(fi, set()).add(cp)

def \_extract\_pairpos\_subtable(subtable, glyph\_to\_cp, raw\_kern):
 """Extract kerning from a PairPos subtable (Format 1 or 2)."""
 if subtable.Format == 1:
 # Individual pairs
 for i, coverage\_glyph in enumerate(subtable.Coverage.glyphs):
 if coverage\_glyph not in glyph\_to\_cp:
 continue
 pair\_set = subtable.PairSet\[i\]
 for pvr in pair\_set.PairValueRecord:
 if pvr.SecondGlyph not in glyph\_to\_cp:
 continue
 xa = 0
 if hasattr(pvr, 'Value1') and pvr.Value1:
 xa = getattr(pvr.Value1, 'XAdvance', 0) or 0
 if xa != 0:
 key = (coverage\_glyph, pvr.SecondGlyph)
 raw\_kern\[key\] = raw\_kern.get(key, 0) + xa
 elif subtable.Format == 2:
 # Class-based pairs
 class\_def1 = subtable.ClassDef1.classDefs if subtable.ClassDef1 else {}
 class\_def2 = subtable.ClassDef2.classDefs if subtable.ClassDef2 else {}
 coverage\_set = set(subtable.Coverage.glyphs)
 for left\_glyph in glyph\_to\_cp:
 if left\_glyph not in coverage\_set:
 continue
 c1 = class\_def1.get(left\_glyph, 0)
 if c1 >= len(subtable.Class1Record):
 continue
 class1\_rec = subtable.Class1Record\[c1\]
 for right\_glyph in glyph\_to\_cp:
 c2 = class\_def2.get(right\_glyph, 0)
 if c2 >= len(class1\_rec.Class2Record):
 continue
 c2\_rec = class1\_rec.Class2Record\[c2\]
 xa = 0
 if hasattr(c2\_rec, 'Value1') and c2\_rec.Value1:
 xa = getattr(c2\_rec.Value1, 'XAdvance', 0) or 0
 if xa != 0:
 key = (left\_glyph, right\_glyph)
 raw\_kern\[key\] = raw\_kern.get(key, 0) + xa

def extract\_kerning\_fonttools(font\_path, codepoints, ppem, pnum\_subs=None):
 """Extract kerning pairs from a font file using fonttools.

 Returns dict of {(leftCp, rightCp): pixel\_adjust} for the given
 codepoints. Values are scaled from font design units to integer
 pixels at ppem.

 When pnum\_subs is provided, substitute glyph names are also included
 in the lookup so kern pairs referencing proportional alternates are found.
 """
 font = TTFont(font\_path)
 units\_per\_em = font\['head'\].unitsPerEm
 cmap = font.getBestCmap() or {}

 # Build glyph\_name -> codepoint map (only for requested codepoints).
 # When pnum is active, include both the original and substitute glyph
 # names so kern pairs referencing either are captured.
 glyph\_to\_cp = {}
 for cp in codepoints:
 gname = cmap.get(cp)
 if gname:
 glyph\_to\_cp\[gname\] = cp
 if pnum\_subs and gname in pnum\_subs:
 glyph\_to\_cp\[pnum\_subs\[gname\]\] = cp

 # Collect raw kerning values in font design units
 raw\_kern = {} # (left\_glyph\_name, right\_glyph\_name) -> design\_units

 # 1\. Legacy kern table
 if 'kern' in font:
 for subtable in font\['kern'\].kernTables:
 if hasattr(subtable, 'kernTable'):
 for (lg, rg), val in subtable.kernTable.items():
 if lg in glyph\_to\_cp and rg in glyph\_to\_cp:
 raw\_kern\[(lg, rg)\] = raw\_kern.get((lg, rg), 0) + val

 # 2\. GPOS 'kern' feature
 if 'GPOS' in font:
 gpos = font\['GPOS'\].table
 kern\_lookup\_indices = set()
 if gpos.FeatureList:
 for fr in gpos.FeatureList.FeatureRecord:
 if fr.FeatureTag == 'kern':
 kern\_lookup\_indices.update(fr.Feature.LookupListIndex)
 for li in kern\_lookup\_indices:
 lookup = gpos.LookupList.Lookup\[li\]
 for st in lookup.SubTable:
 actual = st
 # Unwrap Extension (lookup type 9) wrappers
 if lookup.LookupType == 9 and hasattr(st, 'ExtSubTable'):
 actual = st.ExtSubTable
 if hasattr(actual, 'Format'):
 \_extract\_pairpos\_subtable(actual, glyph\_to\_cp, raw\_kern)

 font.close()

 # Scale design-unit kerning values to 4.4 fixed-point pixels.
 scale = ppem / units\_per\_em
 result = {} # (leftCp, rightCp) -> 4.4 fixed-point adjust
 for (lg, rg), du in raw\_kern.items():
 lcp = glyph\_to\_cp\[lg\]
 rcp = glyph\_to\_cp\[rg\]
 adjust = fp4\_from\_design\_units(du, scale)
 if adjust != 0:
 result\[(lcp, rcp)\] = adjust
 return result

\# The ppem used by the existing glyph rasterization:
\# face.set\_char\_size(size << 6, size << 6, 150, 150)
\# means size\_pt at 150 DPI -> ppem = size \* 150 / 72
ppem = size \* 150.0 / 72.0

kern\_map = {} # (leftCp, rightCp) -> adjust
for face\_idx, cps in face\_idx\_cps.items():
 font\_path = args.fontstack\[face\_idx\]
 subs = pnum\_kern\_subs.get(face\_idx) if args.pnum else None
 kern\_map.update(extract\_kerning\_fonttools(font\_path, cps, ppem, pnum\_subs=subs))

print(f"kerning: {len(kern\_map)} pairs extracted", file=sys.stderr)

\# --- Derive class-based kerning from pairs ---
kern\_left\_classes = \[\] # list of (codepoint, classId)
kern\_right\_classes = \[\] # list of (codepoint, classId)
kern\_matrix = \[\] # flat list of int8\_t values
kern\_left\_class\_count = 0
kern\_right\_class\_count = 0

if kern\_map:
 all\_left\_cps = {lcp for lcp, \_ in kern\_map}
 all\_right\_cps = {rcp for \_, rcp in kern\_map}

 sorted\_right\_cps = sorted(all\_right\_cps)
 sorted\_left\_cps = sorted(all\_left\_cps)

 # Group left codepoints by identical adjustment row
 left\_profile\_to\_class = {}
 left\_class\_map = {}
 left\_class\_id = 1
 for lcp in sorted(all\_left\_cps):
 row = tuple(kern\_map.get((lcp, rcp), 0) for rcp in sorted\_right\_cps)
 if row not in left\_profile\_to\_class:
 left\_profile\_to\_class\[row\] = left\_class\_id
 left\_class\_id += 1
 left\_class\_map\[lcp\] = left\_profile\_to\_class\[row\]

 # Group right codepoints by identical adjustment column
 right\_profile\_to\_class = {}
 right\_class\_map = {}
 right\_class\_id = 1
 for rcp in sorted(all\_right\_cps):
 col = tuple(kern\_map.get((lcp, rcp), 0) for lcp in sorted\_left\_cps)
 if col not in right\_profile\_to\_class:
 right\_profile\_to\_class\[col\] = right\_class\_id
 right\_class\_id += 1
 right\_class\_map\[rcp\] = right\_profile\_to\_class\[col\]

 kern\_left\_class\_count = left\_class\_id - 1
 kern\_right\_class\_count = right\_class\_id - 1

 if kern\_left\_class\_count > 255 or kern\_right\_class\_count > 255:
 print(f"WARNING: kerning class count exceeds uint8\_t range "
 f"(left={kern\_left\_class\_count}, right={kern\_right\_class\_count})",
 file=sys.stderr)

 # Build the class x class matrix
 kern\_matrix = \[0\] \* (kern\_left\_class\_count \* kern\_right\_class\_count)
 for (lcp, rcp), adjust in kern\_map.items():
 lc = left\_class\_map\[lcp\] - 1
 rc = right\_class\_map\[rcp\] - 1
 kern\_matrix\[lc \* kern\_right\_class\_count + rc\] = adjust

 # Build sorted class entry lists
 kern\_left\_classes = sorted(left\_class\_map.items())
 kern\_right\_classes = sorted(right\_class\_map.items())

 matrix\_size = kern\_left\_class\_count \* kern\_right\_class\_count
 entries\_size = (len(kern\_left\_classes) + len(kern\_right\_classes)) \* 3
 print(f"kerning: {kern\_left\_class\_count} left classes, {kern\_right\_class\_count} right classes, "
 f"{matrix\_size + entries\_size} bytes", file=sys.stderr)

\# --- Ligature pair extraction ---
\# Parse the OpenType GSUB table for LigatureSubst (type 4) lookups.
\# Multi-character ligatures (3+ codepoints) are decomposed into chained
\# pairs when an intermediate ligature exists (e.g., ffi = ff + i where ff
\# is itself a ligature). Only pairs where both input codepoints and the
\# output codepoint are in the generated glyph set are included.

all\_codepoints\_set = set(all\_codepoints)

\# Standard Unicode ligature codepoints for known input sequences.
\# Used as a fallback when the GSUB substitute glyph has no cmap entry.
STANDARD\_LIGATURE\_MAP = {
 (0x66, 0x66): 0xFB00, # ff
 (0x66, 0x69): 0xFB01, # fi
 (0x66, 0x6C): 0xFB02, # fl
 (0x66, 0x66, 0x69): 0xFB03, # ffi
 (0x66, 0x66, 0x6C): 0xFB04, # ffl
 (0x17F, 0x74): 0xFB05, # long-s + t
 (0x73, 0x74): 0xFB06, # st
}

def extract\_ligatures\_fonttools(font\_path, codepoints):
 """Extract ligature substitution pairs from a font file using fonttools.

 Returns list of (packed\_pair, ligature\_codepoint) for the given codepoints.
 Multi-character ligatures are decomposed into chained pairs.
 """
 font = TTFont(font\_path)
 cmap = font.getBestCmap() or {}

 # Build glyph\_name -> codepoint and codepoint -> glyph\_name maps
 glyph\_to\_cp = {}
 cp\_to\_glyph = {}
 for cp, gname in cmap.items():
 glyph\_to\_cp\[gname\] = cp
 cp\_to\_glyph\[cp\] = gname

 # Collect raw ligature rules: (sequence\_of\_codepoints) -> ligature\_codepoint
 raw\_ligatures = {} # tuple of codepoints -> ligature codepoint

 if 'GSUB' in font:
 gsub = font\['GSUB'\].table

 # Find lookup indices for ligature features.
 # Currently extracts 'liga' (standard) and 'rlig' (required) only.
 # To also extract discretionary or historical ligatures, add:
 # 'dlig' - Discretionary Ligatures (e.g., ft, st in Noto)
 # 'hlig' - Historical Ligatures (e.g., long-s+t in OpenDyslexic)
 # These are off by default in standard text renderers.
 LIGATURE\_FEATURES = ('liga', 'rlig')
 liga\_lookup\_indices = set()
 if gsub.FeatureList:
 for fr in gsub.FeatureList.FeatureRecord:
 if fr.FeatureTag in LIGATURE\_FEATURES:
 liga\_lookup\_indices.update(fr.Feature.LookupListIndex)

 for li in liga\_lookup\_indices:
 lookup = gsub.LookupList.Lookup\[li\]
 for st in lookup.SubTable:
 actual = st
 # Unwrap Extension (lookup type 7) wrappers
 if lookup.LookupType == 7 and hasattr(st, 'ExtSubTable'):
 actual = st.ExtSubTable
 # LigatureSubst is lookup type 4
 if not hasattr(actual, 'ligatures'):
 continue
 for first\_glyph, ligature\_list in actual.ligatures.items():
 if first\_glyph not in glyph\_to\_cp:
 continue
 first\_cp = glyph\_to\_cp\[first\_glyph\]
 for lig in ligature\_list:
 # lig.Component is a list of subsequent glyph names
 # lig.LigGlyph is the substitute glyph name
 component\_cps = \[\]
 valid = True
 for comp\_glyph in lig.Component:
 if comp\_glyph not in glyph\_to\_cp:
 valid = False
 break
 component\_cps.append(glyph\_to\_cp\[comp\_glyph\])
 if not valid:
 continue
 seq = tuple(\[first\_cp\] + component\_cps)
 if lig.LigGlyph in glyph\_to\_cp:
 lig\_cp = glyph\_to\_cp\[lig.LigGlyph\]
 elif seq in STANDARD\_LIGATURE\_MAP:
 lig\_cp = STANDARD\_LIGATURE\_MAP\[seq\]
 else:
 seq\_str = ', '.join(f'U+{cp:04X}' for cp in seq)
 print(f"ligatures: WARNING: dropping ligature ({seq\_str}) -> "
 f"glyph '{lig.LigGlyph}': output glyph has no cmap entry "
 f"and input sequence is not in STANDARD\_LIGATURE\_MAP",
 file=sys.stderr)
 continue
 raw\_ligatures\[seq\] = lig\_cp

 font.close()

 # Filter: only keep ligatures where all input and output codepoints are
 # in our generated glyph set
 filtered = {}
 for seq, lig\_cp in raw\_ligatures.items():
 if lig\_cp not in codepoints and lig\_cp not in all\_codepoints\_set:
 continue
 if all(cp in codepoints for cp in seq):
 filtered\[seq\] = lig\_cp

 # Decompose into chained pairs
 # For 2-codepoint sequences: direct pair (a, b) -> lig
 # For 3+ codepoint sequences: chain through intermediates
 # e.g., (f, f, i) -> ffi requires (f, f) -> ff to exist,
 # then we add (ff, i) -> ffi
 pairs = \[\]
 # First pass: collect all 2-codepoint ligatures
 two\_char = {seq: lig\_cp for seq, lig\_cp in filtered.items() if len(seq) == 2}
 for seq, lig\_cp in two\_char.items():
 packed = (seq\[0\] << 16) \| seq\[1\]
 pairs.append((packed, lig\_cp))

 # Second pass: decompose 3+ codepoint ligatures into chained pairs
 for seq, lig\_cp in filtered.items():
 if len(seq) < 3:
 continue
 # Try to find an intermediate: check if the first N-1 codepoints
 # form a known ligature, then chain (intermediate, last) -> lig
 prefix = seq\[:-1\]
 last\_cp = seq\[-1\]
 if prefix in filtered:
 intermediate\_cp = filtered\[prefix\]
 packed = (intermediate\_cp << 16) \| last\_cp
 pairs.append((packed, lig\_cp))
 else:
 print(f"ligatures: skipping {len(seq)}-char ligature "
 f"({', '.join(f'U+{cp:04X}' for cp in seq)}) -> U+{lig\_cp:04X}: "
 f"no intermediate ligature for prefix", file=sys.stderr)

 return pairs

ligature\_codepoints = set(cp for cp in all\_codepoints
 if not (COMBINING\_MARKS\_START <= cp <= COMBINING\_MARKS\_END))

\# Map ligature codepoints to the font-stack index that serves them
lig\_cp\_to\_face\_idx = {}
for cp in ligature\_codepoints:
 for face\_idx, f in enumerate(font\_stack):
 if f.get\_char\_index(cp) > 0:
 lig\_cp\_to\_face\_idx\[cp\] = face\_idx
 break

\# Group by face index
lig\_face\_idx\_cps = {}
for cp, fi in lig\_cp\_to\_face\_idx.items():
 lig\_face\_idx\_cps.setdefault(fi, set()).add(cp)

ligature\_pairs = \[\]
for face\_idx, cps in lig\_face\_idx\_cps.items():
 font\_path = args.fontstack\[face\_idx\]
 ligature\_pairs.extend(extract\_ligatures\_fonttools(font\_path, cps))

\# Deduplicate (keep first occurrence) and sort
seen\_lig\_keys = set()
unique\_ligature\_pairs = \[\]
for packed, lig\_cp in ligature\_pairs:
 if packed not in seen\_lig\_keys:
 seen\_lig\_keys.add(packed)
 unique\_ligature\_pairs.append((packed, lig\_cp))
ligature\_pairs = sorted(unique\_ligature\_pairs, key=lambda p: p\[0\])
print(f"ligatures: {len(ligature\_pairs)} pairs extracted", file=sys.stderr)

compress = args.compress

def to\_byte\_aligned(packed, width, height):
 """Convert packed 2-bit bitmap to byte-aligned format (rows padded to byte boundary).

 In packed format, pixels flow continuously across row boundaries (4 pixels/byte).
 In byte-aligned format, each row starts at a byte boundary, padding the last byte
 of each row with zero bits if width % 4 != 0. This improves DEFLATE compression
 because identical pixel rows produce identical byte patterns regardless of position.
 """
 if width == 0 or height == 0:
 return b''
 row\_stride = (width + 3) // 4 # bytes per byte-aligned row
 aligned = bytearray(row\_stride \* height)
 for y in range(height):
 for x in range(width):
 # Read pixel from packed format (continuous bit stream)
 packed\_pos = y \* width + x
 packed\_byte\_idx = packed\_pos // 4
 packed\_shift = (3 - (packed\_pos % 4)) \* 2
 pixel = (packed\[packed\_byte\_idx\] >> packed\_shift) & 0x3

 # Write pixel to byte-aligned format (row-aligned)
 aligned\_byte\_idx = y \* row\_stride + x // 4
 aligned\_shift = (3 - (x % 4)) \* 2
 aligned\[aligned\_byte\_idx\] \|= (pixel << aligned\_shift)
 return bytes(aligned)

\# Build groups for compression
if compress and not is2Bit:
 print("Error: --compress requires --2bit (byte-aligned compression only supports 2-bit format)", file=sys.stderr)
 sys.exit(1)
if compress:
 # Script-based grouping: glyphs that co-occur in typical text rendering
 # are grouped together for efficient LRU caching on the embedded target.
 # Since glyphs are in codepoint order, glyphs in the same Unicode block
 # are contiguous in the array and form natural groups.
 SCRIPT\_GROUP\_RANGES = \[\
 (0x0000, 0x007F), # ASCII\
 (0x0080, 0x00FF), # Latin-1 Supplement\
 (0x0100, 0x017F), # Latin Extended-A\
 (0x0180, 0x024F), # Latin Extended-B\
 (0x0300, 0x036F), # Combining Diacritical Marks\
 (0x0400, 0x04FF), # Cyrillic\
 (0x1EA0, 0x1EF9), # Vietnamese Extended\
 (0x2000, 0x206F), # General Punctuation\
 (0x2070, 0x209F), # Superscripts & Subscripts\
 (0x20A0, 0x20CF), # Currency Symbols\
 (0x2190, 0x21FF), # Arrows\
 (0x2200, 0x22FF), # Math Operators\
 (0xFB00, 0xFB06), # Alphabetic Presentation Forms (ligatures)\
 (0xFFFD, 0xFFFD), # Replacement Character\
 \]

 def get\_script\_group(code\_point):
 for i, (start, end) in enumerate(SCRIPT\_GROUP\_RANGES):
 if start <= code\_point <= end:
 return i
 return -1

 groups = \[\] # list of (first\_glyph\_index, glyph\_count)
 current\_group\_id = None
 group\_start = 0
 group\_count = 0

 for i, (props, packed) in enumerate(all\_glyphs):
 sg = get\_script\_group(props.code\_point)
 if sg != current\_group\_id:
 if group\_count > 0:
 groups.append((group\_start, group\_count))
 current\_group\_id = sg
 group\_start = i
 group\_count = 1
 else:
 group\_count += 1

 if group\_count > 0:
 groups.append((group\_start, group\_count))

 # Compress each group
 compressed\_groups = \[\] # list of (compressed\_bytes, uncompressed\_size, glyph\_count, first\_glyph\_index)
 compressed\_bitmap\_data = \[\]
 compressed\_offset = 0

 # Also build modified glyph props with within-group offsets
 modified\_glyph\_props = list(glyph\_props)

 for first\_idx, count in groups:
 # Concatenate bitmap data for this group
 packed\_len = 0
 group\_aligned = bytearray()
 for gi in range(first\_idx, first\_idx + count):
 props, packed = all\_glyphs\[gi\]
 # Update glyph's dataOffset to be within-group offset (packed offset)
 within\_group\_offset = packed\_len
 old\_props = modified\_glyph\_props\[gi\]
 modified\_glyph\_props\[gi\] = GlyphProps(
 width=old\_props.width,
 height=old\_props.height,
 advance\_x=old\_props.advance\_x,
 left=old\_props.left,
 top=old\_props.top,
 data\_length=old\_props.data\_length,
 data\_offset=within\_group\_offset,
 code\_point=old\_props.code\_point,
 )
 packed\_len += len(packed)
 group\_aligned.extend(to\_byte\_aligned(packed, old\_props.width, old\_props.height))

 # Compress byte-aligned data with raw DEFLATE (no zlib/gzip header)
 compressor = zlib.compressobj(level=9, wbits=-15)
 compressed = compressor.compress(bytes(group\_aligned)) + compressor.flush()

 compressed\_groups.append((compressed, len(group\_aligned), count, first\_idx))
 compressed\_bitmap\_data.extend(compressed)
 compressed\_offset += len(compressed)

 glyph\_props = modified\_glyph\_props
 total\_compressed = len(compressed\_bitmap\_data)
 total\_uncompressed = len(glyph\_data)
 print(f"// Compression: {total\_uncompressed} -> {total\_compressed} bytes ({100\*total\_compressed/total\_uncompressed:.1f}%), {len(groups)} groups", file=sys.stderr)

print(f"""/\*\*
 \\* generated by fontconvert.py
 \\* name: {font\_name}
 \\* size: {size}
 \\* mode: {'2-bit' if is2Bit else '1-bit'}{' compressed: true' if compress else ''}
 \\* Command used: {' '.join(sys.argv)}
 \*/
#pragma once
#include "EpdFontData.h"
""")

if compress:
 print(f"static const uint8\_t {font\_name}Bitmaps\[{len(compressed\_bitmap\_data)}\] = {{")
 for c in chunks(compressed\_bitmap\_data, 16):
 print (" " + " ".join(f"0x{b:02X}," for b in c))
 print ("};\\n");
else:
 print(f"static const uint8\_t {font\_name}Bitmaps\[{len(glyph\_data)}\] = {{")
 for c in chunks(glyph\_data, 16):
 print (" " + " ".join(f"0x{b:02X}," for b in c))
 print ("};\\n");

def cp\_label(cp):
 if cp == 0x5C:
 return ''
 return chr(cp) if 0x20 < cp < 0x7F else f'U+{cp:04X}'

print(f"static const EpdGlyph {font\_name}Glyphs\[\] = {{")
for i, g in enumerate(glyph\_props):
 print (" { " + ", ".join(\[f"{a}" for a in list(g\[:-1\])\]),"},", f"// {cp\_label(g.code\_point)}")
print ("};\\n");

print(f"static const EpdUnicodeInterval {font\_name}Intervals\[\] = {{")
offset = 0
for i\_start, i\_end in intervals:
 print (f" {{ 0x{i\_start:X}, 0x{i\_end:X}, 0x{offset:X} }},")
 offset += i\_end - i\_start + 1
print ("};\\n");

if compress:
 print(f"static const EpdFontGroup {font\_name}Groups\[\] = {{")
 compressed\_offset = 0
 for compressed, uncompressed\_size, count, first\_idx in compressed\_groups:
 print(f" {{ {compressed\_offset}, {len(compressed)}, {uncompressed\_size}, {count}, {first\_idx} }},")
 compressed\_offset += len(compressed)
 print("};\\n")

if kern\_map:
 print(f"static const EpdKernClassEntry {font\_name}KernLeftClasses\[\] = {{")
 for cp, cls in kern\_left\_classes:
 print(f" {{ 0x{cp:04X}, {cls} }}, // {cp\_label(cp)}")
 print("};\\n")

 print(f"static const EpdKernClassEntry {font\_name}KernRightClasses\[\] = {{")
 for cp, cls in kern\_right\_classes:
 print(f" {{ 0x{cp:04X}, {cls} }}, // {cp\_label(cp)}")
 print("};\\n")

 print(f"static const int8\_t {font\_name}KernMatrix\[\] = {{")
 for row in range(kern\_left\_class\_count):
 row\_start = row \* kern\_right\_class\_count
 row\_vals = kern\_matrix\[row\_start:row\_start + kern\_right\_class\_count\]
 print(" " + ", ".join(f"{v:4d}" for v in row\_vals) + ",")
 print("};\\n")

if ligature\_pairs:
 print(f"static const EpdLigaturePair {font\_name}LigaturePairs\[\] = {{")
 for packed\_pair, lig\_cp in ligature\_pairs:
 print(f" {{ 0x{packed\_pair:08X}, 0x{lig\_cp:04X} }}, // {cp\_label(packed\_pair >> 16)} {cp\_label(packed\_pair & 0xFFFF)} -> {cp\_label(lig\_cp)}")
 print("};\\n")

print(f"static const EpdFontData {font\_name} = {{")
print(f" {font\_name}Bitmaps,")
print(f" {font\_name}Glyphs,")
print(f" {font\_name}Intervals,")
print(f" {len(intervals)},")
print(f" {norm\_ceil(face.size.height)},")
print(f" {norm\_ceil(face.size.ascender)},")
print(f" {norm\_floor(face.size.descender)},")
print(f" {'true' if is2Bit else 'false'},")
if compress:
 print(f" {font\_name}Groups,")
 print(f" {len(compressed\_groups)},")
else:
 print(" nullptr,")
 print(" 0,")
\# glyphToGroup (not used for script-grouped fonts)
print(" nullptr,")
if kern\_map:
 print(f" {font\_name}KernLeftClasses,")
 print(f" {font\_name}KernRightClasses,")
 print(f" {font\_name}KernMatrix,")
 print(f" {len(kern\_left\_classes)},")
 print(f" {len(kern\_right\_classes)},")
 print(f" {kern\_left\_class\_count},")
 print(f" {kern\_right\_class\_count},")
else:
 print(f" nullptr,")
 print(f" nullptr,")
 print(f" nullptr,")
 print(f" 0,")
 print(f" 0,")
 print(f" 0,")
 print(f" 0,")
if ligature\_pairs:
 print(f" {font\_name}LigaturePairs,")
 print(f" {len(ligature\_pairs)},")
else:
 print(f" nullptr,")
 print(f" 0,")
print("};")
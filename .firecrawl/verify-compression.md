#!/usr/bin/env python3
"""
Round-trip verification for compressed font headers.

Parses each generated .h file in the given directory, identifies compressed fonts
(those with a Groups array), decompresses each group (byte-aligned bitmap format),
compacts to packed format, and verifies the data matches expected glyph sizes.

Supports both contiguous-group fonts (Latin) and frequency-grouped fonts (CJK)
with glyphToGroup mapping arrays.
"""
import math
import os
import re
import sys
import zlib

def parse\_hex\_array(text):
 """Extract bytes from a C hex array string like '{ 0xAB, 0xCD, ... }'"""
 hex\_vals = re.findall(r'0x(\[0-9A-Fa-f\]{2})', text)
 return bytes(int(h, 16) for h in hex\_vals)

def parse\_uint8\_array(text):
 """Extract uint8/uint16 values from a C array string like '{ 0, 1, 0xFF, ... }'"""
 return \[int(v, 0) for v in re.findall(r'\\b0x\[0-9A-Fa-f\]+\\b\|\\b\\d+\\b', text)\]

def parse\_groups(text):
 """Parse EpdFontGroup array entries: { compressedOffset, compressedSize, uncompressedSize, glyphCount, firstGlyphIndex }"""
 groups = \[\]
 for match in re.finditer(r'\\{\\s\*(\\d+)\\s\*,\\s\*(\\d+)\\s\*,\\s\*(\\d+)\\s\*,\\s\*(\\d+)\\s\*,\\s\*(\\d+)\\s\*\\}', text):
 groups.append({
 'compressedOffset': int(match.group(1)),
 'compressedSize': int(match.group(2)),
 'uncompressedSize': int(match.group(3)),
 'glyphCount': int(match.group(4)),
 'firstGlyphIndex': int(match.group(5)),
 })
 return groups

def parse\_glyphs(text):
 """Parse EpdGlyph array entries: { width, height, advanceX, left, top, dataLength, dataOffset }"""
 glyphs = \[\]
 for match in re.finditer(r'\\{\\s\*(-?\\d+)\\s\*,\\s\*(-?\\d+)\\s\*,\\s\*(-?\\d+)\\s\*,\\s\*(-?\\d+)\\s\*,\\s\*(-?\\d+)\\s\*,\\s\*(-?\\d+)\\s\*,\\s\*(-?\\d+)\\s\*\\}', text):
 glyphs.append({
 'width': int(match.group(1)),
 'height': int(match.group(2)),
 'advanceX': int(match.group(3)),
 'left': int(match.group(4)),
 'top': int(match.group(5)),
 'dataLength': int(match.group(6)),
 'dataOffset': int(match.group(7)),
 })
 return glyphs

def get\_group\_glyph\_indices(group, group\_index, glyphs, glyph\_to\_group):
 """Get the ordered list of glyph indices belonging to a group."""
 if glyph\_to\_group is not None:
 # Frequency-grouped: scan all glyphs
 return \[i for i in range(len(glyphs)) if glyph\_to\_group\[i\] == group\_index\]
 else:
 # Contiguous: sequential from firstGlyphIndex
 first = group\['firstGlyphIndex'\]
 return list(range(first, first + group\['glyphCount'\]))

def compact\_aligned\_to\_packed(aligned\_data, width, height):
 """Convert byte-aligned 2-bit bitmap to packed format (reverse of to\_byte\_aligned).

 In byte-aligned format, each row starts at a byte boundary.
 In packed format, pixels flow continuously across row boundaries (4 pixels/byte).
 """
 if width == 0 or height == 0:
 return b''
 packed\_size = math.ceil(width \* height / 4)
 packed = bytearray(packed\_size)
 row\_stride = (width + 3) // 4 # bytes per byte-aligned row

 for y in range(height):
 for x in range(width):
 # Read pixel from byte-aligned format (row-aligned)
 aligned\_byte\_idx = y \* row\_stride + x // 4
 aligned\_shift = (3 - (x % 4)) \* 2
 pixel = (aligned\_data\[aligned\_byte\_idx\] >> aligned\_shift) & 0x3

 # Write pixel to packed format (continuous bit stream)
 packed\_pos = y \* width + x
 packed\_byte\_idx = packed\_pos // 4
 packed\_shift = (3 - (packed\_pos % 4)) \* 2
 packed\[packed\_byte\_idx\] \|= (pixel << packed\_shift)

 return bytes(packed)

def verify\_font\_file(filepath):
 """Verify a single font header file. Returns (font\_name, success, message)."""
 with open(filepath, 'r') as f:
 content = f.read()

 # Check if this is a compressed font (has Groups array)
 groups\_match = re.search(r'static const EpdFontGroup (\\w+)Groups\\\[\\\]', content)
 if not groups\_match:
 return (os.path.basename(filepath), None, "uncompressed, skipping")

 font\_name = groups\_match.group(1)

 # Extract bitmap data
 bitmap\_match = re.search(
 r'static const uint8\_t ' + re.escape(font\_name) + r'Bitmaps\\\[\\d+\\\]\\s\*=\\s\*\\{(\[^}\]+)\\}',
 content, re.DOTALL
 )
 if not bitmap\_match:
 return (font\_name, False, "could not find Bitmaps array")

 compressed\_data = parse\_hex\_array(bitmap\_match.group(1))

 # Extract groups
 groups\_array\_match = re.search(
 r'static const EpdFontGroup ' + re.escape(font\_name) + r'Groups\\\[\\\]\\s\*=\\s\*\\{(.+?)\\};',
 content, re.DOTALL
 )
 if not groups\_array\_match:
 return (font\_name, False, "could not find Groups array")

 groups = parse\_groups(groups\_array\_match.group(1))
 if not groups:
 return (font\_name, False, "Groups array parsed to 0 entries; check format")

 # Extract glyphs
 glyphs\_match = re.search(
 r'static const EpdGlyph ' + re.escape(font\_name) + r'Glyphs\\\[\\\]\\s\*=\\s\*\\{(.+?)\\};',
 content, re.DOTALL
 )
 if not glyphs\_match:
 return (font\_name, False, "could not find Glyphs array")

 glyphs = parse\_glyphs(glyphs\_match.group(1))

 # Check for glyphToGroup array (frequency-grouped fonts)
 glyph\_to\_group = None
 g2g\_match = re.search(
 r'static const uint16\_t ' + re.escape(font\_name) + r'GlyphToGroup\\\[\\\]\\s\*=\\s\*\\{(.+?)\\};',
 content, re.DOTALL
 )
 if g2g\_match:
 glyph\_to\_group = parse\_uint8\_array(g2g\_match.group(1))
 if len(glyph\_to\_group) != len(glyphs):
 return (font\_name, False, f"glyphToGroup length ({len(glyph\_to\_group)}) != glyph count ({len(glyphs)})")
 max\_group\_id = max(glyph\_to\_group)
 if max\_group\_id >= len(groups):
 return (font\_name, False, f"glyphToGroup contains group ID {max\_group\_id} but only {len(groups)} groups exist")

 # Verify each group
 for gi, group in enumerate(groups):
 # Extract compressed chunk
 chunk = compressed\_data\[group\['compressedOffset'\]:group\['compressedOffset'\] + group\['compressedSize'\]\]
 if len(chunk) != group\['compressedSize'\]:
 return (font\_name, False, f"group {gi}: compressed data truncated (expected {group\['compressedSize'\]}, got {len(chunk)})")

 # Decompress with raw DEFLATE — result is byte-aligned data
 try:
 decompressed = zlib.decompress(chunk, -15)
 except zlib.error as e:
 return (font\_name, False, f"group {gi}: decompression failed: {e}")

 if len(decompressed) != group\['uncompressedSize'\]:
 return (font\_name, False, f"group {gi}: size mismatch (expected {group\['uncompressedSize'\]}, got {len(decompressed)})")

 # Get glyph indices for this group
 group\_glyph\_indices = get\_group\_glyph\_indices(group, gi, glyphs, glyph\_to\_group)
 if glyph\_to\_group is not None and len(group\_glyph\_indices) != group\['glyphCount'\]:
 return (font\_name, False,
 f"group {gi}: glyphCount {group\['glyphCount'\]} != mapping count {len(group\_glyph\_indices)}")

 # Walk through byte-aligned data, compact each glyph, and verify against packed format
 byte\_aligned\_offset = 0
 packed\_offset = 0

 for glyph\_idx in group\_glyph\_indices:
 if glyph\_idx >= len(glyphs):
 return (font\_name, False, f"group {gi}: glyph index {glyph\_idx} out of range")
 glyph = glyphs\[glyph\_idx\]
 width = glyph\['width'\]
 height = glyph\['height'\]

 if width == 0 or height == 0:
 # Zero-size glyphs should have dataOffset == current packed\_offset and dataLength == 0
 if glyph\['dataOffset'\] != packed\_offset:
 return (font\_name, False, f"group {gi}, glyph {glyph\_idx}: zero-size glyph dataOffset {glyph\['dataOffset'\]} != expected packed offset {packed\_offset}")
 if glyph\['dataLength'\] != 0:
 return (font\_name, False, f"group {gi}, glyph {glyph\_idx}: zero-size glyph dataLength {glyph\['dataLength'\]} != expected 0")
 continue

 aligned\_size = ((width + 3) // 4) \* height
 packed\_size = math.ceil(width \* height / 4)

 # Verify packed offset and size match glyph metadata
 if glyph\['dataOffset'\] != packed\_offset:
 return (font\_name, False, f"group {gi}, glyph {glyph\_idx}: dataOffset {glyph\['dataOffset'\]} != expected packed offset {packed\_offset}")
 if glyph\['dataLength'\] != packed\_size:
 return (font\_name, False, f"group {gi}, glyph {glyph\_idx}: dataLength {glyph\['dataLength'\]} != expected packed length {packed\_size} "
 f"(width={width}, height={height})")

 # Extract byte-aligned data for this glyph
 if byte\_aligned\_offset + aligned\_size > len(decompressed):
 return (font\_name, False, f"group {gi}, glyph {glyph\_idx}: byte-aligned data extends beyond decompressed buffer "
 f"(offset={byte\_aligned\_offset}, size={aligned\_size}, buf\_size={len(decompressed)})")

 aligned\_glyph = decompressed\[byte\_aligned\_offset:byte\_aligned\_offset + aligned\_size\]

 # Compact to packed and verify pixel values are valid (0-3 for 2-bit)
 packed\_glyph = compact\_aligned\_to\_packed(aligned\_glyph, width, height)
 if len(packed\_glyph) != packed\_size:
 return (font\_name, False, f"group {gi}, glyph {glyph\_idx}: compacted size {len(packed\_glyph)} != expected {packed\_size}")

 byte\_aligned\_offset += aligned\_size
 packed\_offset += packed\_size

 # Verify total byte-aligned size matches uncompressedSize
 if byte\_aligned\_offset != group\['uncompressedSize'\]:
 return (font\_name, False, f"group {gi}: total byte-aligned size {byte\_aligned\_offset} != uncompressedSize {group\['uncompressedSize'\]}")

 extra\_info = ""
 if glyph\_to\_group is not None:
 extra\_info = " (frequency-grouped)"
 return (font\_name, True, f"{len(groups)} groups, {len(glyphs)} glyphs OK{extra\_info}")

def main():
 if len(sys.argv) < 2:
 print(f"Usage: {sys.argv\[0\]} ", file=sys.stderr)
 sys.exit(1)

 font\_dir = sys.argv\[1\]
 if not os.path.isdir(font\_dir):
 print(f"Error: {font\_dir} is not a directory", file=sys.stderr)
 sys.exit(1)

 files = sorted(f for f in os.listdir(font\_dir) if f.endswith('.h') and f != 'all.h')
 passed = 0
 failed = 0
 skipped = 0

 for filename in files:
 filepath = os.path.join(font\_dir, filename)
 \_font\_name, success, message = verify\_font\_file(filepath)

 if success is None:
 skipped += 1
 elif success:
 passed += 1
 print(f" PASS: {filename} ({message})")
 else:
 failed += 1
 print(f" FAIL: {filename} - {message}")

 print(f"\\nResults: {passed} passed, {failed} failed, {skipped} skipped (uncompressed)")

 if failed > 0:
 sys.exit(1)

if \_\_name\_\_ == '\_\_main\_\_':
 main()
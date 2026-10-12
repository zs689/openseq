#!/usr/bin/env python3
"""Generate OpenSeq's app icons with nothing but the standard library.

Writes `src-tauri/icons/{32x32,128x128,icon}.png` and `icon.ico`.
Run from the project root:  python scripts/make_icons.py
"""

from __future__ import annotations

import os
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ICON_DIR = os.path.join(ROOT, "src-tauri", "icons")

BG_TOP = (76, 139, 245)
BG_BOTTOM = (47, 111, 221)
INK = (255, 255, 255)


def _blend(a, b, t):
    return tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))


def _rounded_alpha(x, y, w, h, radius):
    """Anti-aliased coverage for a rounded rectangle covering the whole tile."""
    cx = min(max(x + 0.5, radius), w - radius)
    cy = min(max(y + 0.5, radius), h - radius)
    dx = (x + 0.5) - cx
    dy = (y + 0.5) - cy
    dist = (dx * dx + dy * dy) ** 0.5
    return max(0.0, min(1.0, radius + 0.5 - dist))


def pixel(x, y, size):
    t = y / max(1, size - 1)
    r, g, b = _blend(BG_TOP, BG_BOTTOM, t)

    # The mark: a vertical bar plus a small block at its lower right (◧).
    u = x / size
    v = y / size
    on_bar = 0.28 <= u <= 0.40 and 0.22 <= v <= 0.78
    on_block = 0.48 <= u <= 0.72 and 0.48 <= v <= 0.78
    if on_bar or on_block:
        r, g, b = INK

    a = _rounded_alpha(x, y, size, size, size * 0.22)
    return bytes((r, g, b, round(a * 255)))


def png(size):
    raw = bytearray()
    for y in range(size):
        raw.append(0)  # filter: none
        for x in range(size):
            raw.extend(pixel(x, y, size))

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    ihdr = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)  # 8-bit RGBA
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + chunk(b"IEND", b"")
    )


def ico(images):
    """images: list of (size, png_bytes)."""
    count = len(images)
    header = struct.pack("<HHH", 0, 1, count)
    offset = 6 + 16 * count
    entries = b""
    blobs = b""
    for size, blob in images:
        dim = 0 if size >= 256 else size
        entries += struct.pack(
            "<BBBBHHII", dim, dim, 0, 0, 1, 32, len(blob), offset + len(blobs)
        )
        blobs += blob
    return header + entries + blobs


def main() -> None:
    os.makedirs(ICON_DIR, exist_ok=True)
    for size, name in [(32, "32x32.png"), (128, "128x128.png"), (512, "icon.png")]:
        with open(os.path.join(ICON_DIR, name), "wb") as fh:
            fh.write(png(size))
    with open(os.path.join(ICON_DIR, "icon.ico"), "wb") as fh:
        fh.write(ico([(s, png(s)) for s in (16, 32, 48, 64, 128, 256)]))
    print("icons written to", ICON_DIR)


if __name__ == "__main__":
    main()

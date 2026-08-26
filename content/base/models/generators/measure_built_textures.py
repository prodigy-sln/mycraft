"""Measure the built texture set's colours, independently of every crate.

Provenance, not a build step. Nothing runs this — not the build, not the gate,
not the client — and no test asserts a figure it prints. It is kept because the
figures Phase 9 rests on were offline-computed, and **the durable form of a
measurement is the command that reproduces it**: a record naming a program that
no longer exists reads as reproducible and is not.

**That was a decision on principle and it can now be made on evidence.** Two
figures in Phase 9's records condensed a set into its extremum — one grass side's
share stated as all four's, and a sibling percentage stated from one side. Both
were found by somebody **re-running this program while writing an unrelated
documentation section**, not by anybody checking them. That is
`docs/technical/testing.md`'s "second check on whatever figure is load-bearing"
arriving as a side effect of the artefact simply being *runnable*, from a
direction nobody planned — and a prose-only record of the method could not have
produced it. Keeping it costs a file nothing executes; not keeping it would have
cost two wrong numbers standing in the tree.

**Why a third program.** Two others had already measured this set. The first
copied `crates/mc-testkit/src/frame/color.rs` verbatim, deliberately, so that its
distances would predict what `probe.rs` computes. The second called the shipped
`compare`, `chain` and `placeholder_mean_color` through path dependencies, which
is the right check for "does this predict the probe" and no check at all on the
colour maths itself — both routes run the same CIELAB conversion. This one shares
no line with either: the PNG reader, the sRGB transfer function, the sRGB -> XYZ
(D65) matrix and CIE76 are all written here from the published formulae. Every
figure the three have in common agrees to the last printed decimal, which is what
establishes the colour maths is *right* rather than merely consistent.

Run it from a checkout that has built its art:

    cargo run -p voxforge -- build content/base/textures.toml
    python content/base/models/generators/measure_built_textures.py

It needs no third-party package. Python's own `zlib` decompresses the image data
and the five PNG filter types are undone below by hand.

What it prints, per texture: the distinct texel colours and how many, the mean in
linear light and the mean over stored bytes and how far apart those two stand,
the furthest texel from the linear mean, and the share of texels within delta-E
10 of it. Then every pairwise distance between the eight, and the three generated
means the art replaces.

**Read the output as a dated observation, never as a contract.** It was taken at
`d2a342f` against the set the shipped manifest bakes; the numbers are recorded in
that spec's `test-map.md` beside a note saying which of them have a second route
and which have none.
"""

import glob
import os
import struct
import sys
import zlib

# The built set sits beside the manifest that produced it, two directories up
# from this script. Derived rather than typed, so a checkout anywhere works.
SET = os.path.join(
    os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
    "textures",
)


def decode_png(path):
    """The width, height and RGBA texels of an 8-bit non-interlaced PNG."""
    data = open(path, "rb").read()
    assert data[:8] == b"\x89PNG\r\n\x1a\n", path
    at = 8
    idat = b""
    width = height = channels = None
    while at < len(data):
        (length,) = struct.unpack(">I", data[at : at + 4])
        kind = data[at + 4 : at + 8]
        body = data[at + 8 : at + 8 + length]
        if kind == b"IHDR":
            width, height, depth, color, _comp, _filt, interlaced = struct.unpack(
                ">IIBBBBB", body
            )
            assert depth == 8 and interlaced == 0, (depth, interlaced)
            channels = {0: 1, 2: 3, 4: 2, 6: 4}[color]
        elif kind == b"IDAT":
            idat += body
        at += 12 + length
    raw = zlib.decompress(idat)
    stride = width * channels
    out = []
    previous = bytearray(stride)
    at = 0
    for _ in range(height):
        kind = raw[at]
        line = bytearray(raw[at + 1 : at + 1 + stride])
        at += 1 + stride
        undo_filter(kind, line, previous, channels, stride)
        previous = line
        for x in range(width):
            texel = line[x * channels : (x + 1) * channels]
            if channels == 4:
                out.append(tuple(texel))
            elif channels == 3:
                out.append((texel[0], texel[1], texel[2], 255))
            else:
                raise AssertionError(channels)
    return width, height, out


def undo_filter(kind, line, previous, channels, stride):
    """Reverse one scanline's filter, in place. All five types."""
    for i in range(stride):
        left = line[i - channels] if i >= channels else 0
        up = previous[i]
        up_left = previous[i - channels] if i >= channels else 0
        x = line[i]
        if kind == 0:
            value = x
        elif kind == 1:
            value = x + left
        elif kind == 2:
            value = x + up
        elif kind == 3:
            value = x + ((left + up) >> 1)
        elif kind == 4:
            value = x + paeth(left, up, up_left)
        else:
            raise AssertionError(kind)
        line[i] = value & 0xFF


def paeth(left, up, up_left):
    estimate = left + up - up_left
    to_left, to_up, to_corner = (
        abs(estimate - left),
        abs(estimate - up),
        abs(estimate - up_left),
    )
    if to_left <= to_up and to_left <= to_corner:
        return left
    return up if to_up <= to_corner else up_left


# IEC 61966-2-1, both directions, written from the specification.
def to_linear(stored):
    encoded = stored / 255.0
    if encoded <= 0.04045:
        return encoded / 12.92
    return ((encoded + 0.055) / 1.055) ** 2.4


def to_stored(linear):
    if linear <= 0.0031308:
        encoded = 12.92 * linear
    else:
        encoded = 1.055 * (linear ** (1 / 2.4)) - 0.055
    return max(0, min(255, int(round(encoded * 255.0))))


# sRGB's own D65 primaries, and the white point CIELAB is relative to.
MATRIX = (
    (0.4124564, 0.3575761, 0.1804375),
    (0.2126729, 0.7151522, 0.0721750),
    (0.0193339, 0.1191920, 0.9503041),
)
WHITE = (0.95047, 1.00000, 1.08883)


def lab(rgb):
    linear = [to_linear(channel) for channel in rgb]
    xyz = [sum(MATRIX[i][j] * linear[j] for j in range(3)) for i in range(3)]

    def f(t):
        return t ** (1 / 3) if t > (6 / 29) ** 3 else t / (3 * (6 / 29) ** 2) + 4 / 29

    fx, fy, fz = (f(xyz[i] / WHITE[i]) for i in range(3))
    return (116 * fy - 16, 500 * (fx - fy), 200 * (fy - fz))


def delta_e(one, other):
    """CIE76: the Euclidean distance between two colours in CIELAB."""
    a, b = lab(one), lab(other)
    return sum((a[i] - b[i]) ** 2 for i in range(3)) ** 0.5


def linear_mean(texels):
    count = len(texels)
    return tuple(
        to_stored(sum(to_linear(t[c]) for t in texels) / count) for c in range(3)
    )


def byte_mean(texels):
    count = len(texels)
    return tuple(int(round(sum(t[c] for t in texels) / count)) for c in range(3))


# `placeholder_mean_color`, written out: FNV-1a 64 over the key's text, each of
# the low three bytes scaled into `40 + (byte * 176) >> 8`.
FNV_BASIS = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3


def generated_mean(key):
    hashed = FNV_BASIS
    for byte in key.encode():
        hashed = ((hashed ^ byte) * FNV_PRIME) & 0xFFFFFFFFFFFFFFFF
    return tuple(
        (40 + (((hashed >> (channel * 8)) & 0xFF) * 176 >> 8)) & 0xFF
        for channel in range(3)
    )


def key_of(path):
    return os.path.basename(path)[: -len(".png")].replace("__", ":")


def main():
    images = sorted(glob.glob(os.path.join(SET, "*.png")))
    if not images:
        print(
            f"no images under {SET} — the set is derived and is never committed, so run\n"
            "  cargo run -p voxforge -- build content/base/textures.toml\n"
            "and try again",
            file=sys.stderr,
        )
        return 1

    means = {}
    print(
        f"{'key':26} {'size':8} {'#c':>3} {'linear mean':18} {'byte mean':18} "
        f"{'dE':>5} {'far':>6} {'in10':>7}"
    )
    for path in images:
        key = key_of(path)
        width, height, texels = decode_png(path)
        rgb = [texel[:3] for texel in texels]
        distinct = sorted(set(rgb))
        light, stored = linear_mean(rgb), byte_mean(rgb)
        means[key] = light
        furthest = max(delta_e(texel, light) for texel in rgb)
        within = sum(1 for texel in rgb if delta_e(texel, light) <= 10.0) / len(rgb)
        print(
            f"{key:26} {width}x{height:<5} {len(distinct):>3} {str(light):18} "
            f"{str(stored):18} {delta_e(light, stored):5.2f} {furthest:6.2f} "
            f"{within * 100:6.2f}%"
        )

    print("\npairwise delta-E over linear means")
    keys = sorted(means)
    for index, one in enumerate(keys):
        for other in keys[index + 1 :]:
            print(f"  {one:24} vs {other:24} {delta_e(means[one], means[other]):6.2f}")

    print("\ngenerated means against the art that replaces them")
    for generated, art in (
        ("base:dirt", "base:dirt"),
        ("base:grass", "base:grass_top"),
        ("base:stone", "base:stone"),
    ):
        stand_in = generated_mean(generated)
        print(
            f"  {generated:12} generated {str(stand_in):18} vs {art:16} "
            f"{str(means[art]):18} dE {delta_e(stand_in, means[art]):6.2f}"
        )

    print("\ntexel colours, most common first")
    for path in images:
        _width, _height, texels = decode_png(path)
        rgb = [texel[:3] for texel in texels]
        counts = {}
        for texel in rgb:
            counts[texel] = counts.get(texel, 0) + 1
        ordered = sorted(counts.items(), key=lambda pair: -pair[1])
        shares = ", ".join(
            f"{colour} {count * 100 / len(rgb):.1f}%" for colour, count in ordered
        )
        print(f"  {key_of(path)}: {shares}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

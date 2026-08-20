"""Is each baked image the model plane it claims to be, and in which orientation?

A second, independent route to the figures the orientation tests assert. It
reads the `.mcvox` by hand, builds each face's plane from first principles, and
scores all eight dihedral transforms against the built PNG in disagreeing
texels. `crates/mc-client/tests/support/model.rs` does the same thing in Rust;
these two share no code, and running both is how the *bottom* face's mapping was
corrected — the first hand-derivation of it was wrong, and only a measurement
said so.

Kept beside the generators for the reason the measurement program next door is:
what a figure in a test or a document was measured with must not live in a
temporary path. The header of `measure_built_textures.py` argues that on
evidence; this file inherits the argument and adds one more instance to it.

# The mapping, and why it is derived rather than tabulated

`(right, up, normal)` is a right-handed orthonormal triple. `normal` is the
face's outward direction. For the four sides the image runs up the model, so
`up` is world +y and `right = up x normal`. For the two plan views there is no
`y` in the picture and the documented column axis is `x`, so `right` is +x and
`up = normal x right`.

Four tabulated cases would each be wrong on their own, which is exactly what
happened to the renderer: `mc_render::geometry::PLANE_AXES` is such a table and
five of its six rows were wrong.

Run it from anywhere:

    python content/base/models/generators/measure_face_orientation.py
"""

import glob
import importlib.util
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
BASE = os.path.join(ROOT, "content", "base")
PROGRAM = os.path.join(HERE, "measure_built_textures.py")

# One decoder, not two: the PNG reader beside this file is already the
# independent one, written from the published format rather than from the
# encoder.
spec = importlib.util.spec_from_file_location("measured", PROGRAM)
measured = importlib.util.module_from_spec(spec)
sys.modules["measured"] = measured
spec.loader.exec_module(measured)

# Which compass side each face word shows, as content/base/textures.toml
# records it: `front` looks along -z and so shows +z, which is south.
NORMALS = {
    "front": (0, 0, 1),
    "back": (0, 0, -1),
    "right": (1, 0, 0),
    "left": (-1, 0, 0),
    "top": (0, 1, 0),
    "bottom": (0, -1, 0),
}


def cross(left, right):
    """The cross product of two axis-aligned unit vectors."""
    return (
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    )


def basis(face):
    """The image's (right, down) directions for `face`, in model coordinates."""
    normal = NORMALS[face]
    up = (0, 1, 0) if normal[1] == 0 else cross(normal, (1, 0, 0))
    right = cross(up, normal)
    return right, tuple(-component for component in up)


def declared_palette():
    """Every material the content declares, by name, as (r, g, b)."""
    found = {}
    for path in glob.glob(os.path.join(BASE, "materials", "*.toml")):
        with open(path, encoding="utf-8") as file:
            text = file.read()
        name = re.search(r'^\s*name\s*=\s*"([^"]+)"', text, re.M)
        color = re.search(r'^\s*color\s*=\s*"#([0-9a-fA-F]{6})', text, re.M)
        if name and color:
            hexed = color.group(1)
            found[name.group(1)] = tuple(int(hexed[at:at + 2], 16) for at in (0, 2, 4))
    return found


def read_model(path):
    """A model's voxels as {(x, y, z): material}, and its edge length."""
    with open(path, encoding="utf-8") as file:
        text = file.read()
    palette = dict(re.findall(r'^"(.)"\s*=\s*"([^"]+)"', text, re.M))
    edge = int(re.search(r"^size\s*=\s*\[\s*(\d+)", text, re.M).group(1))
    if re.search(r'^slice\s*=\s*"(\w)"', text, re.M).group(1) != "y":
        raise SystemExit(f"{path} is not sliced on y, and this reader only knows that")
    voxels = {}
    for block in re.finditer(
        r'^\[\[layers\]\].*?^y\s*=\s*(\d+).*?grid\s*=\s*"""\n(.*?)"""', text, re.M | re.S
    ):
        y = int(block.group(1))
        rows = [row for row in block.group(2).split("\n") if row.strip()]
        for z, row in enumerate(rows):
            for x, spelled in enumerate(row):
                voxels[(x, y, z)] = palette[spelled]
    if len(voxels) != edge ** 3:
        raise SystemExit(f"{path} states {len(voxels)} voxels, not {edge ** 3}")
    return voxels, edge


def plane(voxels, edge, face):
    """The outermost plane `face` shows, as grid[row][column] of material names."""
    last = edge - 1
    normal = NORMALS[face]
    right, down = basis(face)

    def coordinate(axis, column, row):
        if normal[axis]:
            return last if normal[axis] == 1 else 0
        if right[axis]:
            return column if right[axis] == 1 else last - column
        return row if down[axis] == 1 else last - row

    return [
        [voxels[tuple(coordinate(axis, column, row) for axis in (0, 1, 2))]
         for column in range(edge)]
        for row in range(edge)
    ]


def transforms(grid):
    """The eight dihedral transforms of a square grid, identity first."""
    edge = len(grid)

    def turned(inner):
        return [[inner[edge - 1 - c][r] for c in range(edge)] for r in range(edge)]

    def mirror(inner):
        return [list(reversed(row)) for row in inner]

    out = []
    at = grid
    for quarter in range(4):
        out.append((f"rot{quarter * 90}", at))
        out.append((f"rot{quarter * 90}+mirror", mirror(at)))
        at = turned(at)
    return out


def baked_from(model_file):
    """Every (face word, key) the manifest bakes from `model_file`."""
    with open(os.path.join(BASE, "textures.toml"), encoding="utf-8") as file:
        text = file.read()
    found = []
    for entry in text.split("[[texture]]")[1:]:
        stated = dict(re.findall(r'^\s*(\w+)\s*=\s*"([^"]+)"', entry, re.M))
        if stated.get("model", "").endswith(model_file) and "face" in stated:
            found.append((stated["face"], stated["key"]))
    return found


def main():
    declared = declared_palette()
    model_file = "grass-block.mcvox"
    voxels, edge = read_model(os.path.join(BASE, "models", model_file))
    for face, key in baked_from(model_file):
        path = os.path.join(BASE, "textures", key.replace(":", "__") + ".png")
        width, height, texels = measured.decode_png(path)
        image = [[texels[y * width + x][:3] for x in range(width)] for y in range(height)]
        expected = plane(voxels, edge, face)
        print(f"--- {face:6} -> {key}  ({width}x{height})")
        for name, turned in transforms(expected):
            wrong = sum(
                1
                for r in range(edge)
                for c in range(edge)
                if declared[turned[r][c]] != image[r][c]
            )
            print(f"      {name:>16}  disagreeing texels: {wrong:4}")
        mirrored = sum(
            1
            for r in range(edge)
            for c in range(edge)
            if image[r][c] != image[r][edge - 1 - c]
        )
        print(f"      texels differing from the image's own horizontal mirror: {mirrored}")


if __name__ == "__main__":
    main()

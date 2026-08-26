"""Generate the water block's sparse speckle, and say how sparse it came out.

Provenance, not a build step. This script ran once and produced the model the
repository tracks. That model is the source of truth for the art: nothing in the
build, the gate or the client runs this script, a contributor never needs to,
and the model is what a review reads.

**It is a sibling of `gen_stone.py` and deliberately a much simpler one.** Stone
searches a salt for a noise field with no directional grain, because rock is
where a repeated tile reads as a pattern. A sea surface has no authored shape the
way a sod line does and its three tones stand only ΔE 6.29 apart at the widest,
so a grain nobody can see is not worth a search. What is chosen here is the
density, and it is chosen for one reason: **water is the smoothest surface in
this set**, so it takes the fewest accents of anything in it.

Unlike `gen_stone.py` this writes the file the repository actually tracks. It is
still not a way to regenerate the model — re-running it with a different constant
below would silently change the art under a manifest that would go on folding to
a new value and calling every checkout stale.
"""

SIZE = 16

# Rates, not counts. A tone is picked per voxel from a hash of its position, so
# what is stated is the share of the block each accent takes: 5% each, against
# stone's 19% and 15%. On a 16x16 face that is thirteen texels of each accent
# among two hundred and thirty of the base.
DARK_ABOVE = 0.90
LIGHT_ABOVE = 0.95

# Fixed, and not searched for. See this module's docstring: there is no grain to
# search away at this separation.
SALT = 0x4C799E


def mix(*parts):
    h = 0xCBF29CE484222325
    for value in parts:
        h ^= (value + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
        h ^= h >> 29
        h = (h * 0xFF51AFD7ED558CCD) & 0xFFFFFFFFFFFFFFFF
        h ^= h >> 32
    return h


def unit(*parts):
    return mix(*parts) / 0xFFFFFFFFFFFFFFFF


def tone(x, y, z, salt):
    r = unit(x, y, z, salt)
    if r < DARK_ABOVE:
        return "w"
    if r < LIGHT_ABOVE:
        return "W"
    return "l"


def build(salt):
    grid = [[[tone(x, y, z, salt) for x in range(SIZE)] for z in range(SIZE)] for y in range(SIZE)]

    # Every face of this block tiles against a copy of itself, so the last row
    # and column of a face repeat its first. Stone does the same and for the same
    # reason: a seam is the one artefact a viewer picks out of noise instantly.
    for y in range(SIZE):
        grid[y][0][SIZE - 1] = grid[y][0][0]
        grid[y][SIZE - 1][0] = grid[y][0][0]
        grid[y][SIZE - 1][SIZE - 1] = grid[y][0][0]
    for y in (0, SIZE - 1):
        for z in range(SIZE - 1):
            grid[y][z][SIZE - 1] = grid[y][z][0]
        grid[y][SIZE - 1] = list(grid[y][0])
    for z in range(SIZE):
        grid[SIZE - 1][z][0] = grid[0][z][0]
        grid[SIZE - 1][z][SIZE - 1] = grid[0][z][SIZE - 1]
    for x in range(SIZE):
        grid[SIZE - 1][0][x] = grid[0][0][x]
        grid[SIZE - 1][SIZE - 1][x] = grid[0][SIZE - 1][x]

    return grid


grid = build(SALT)

top = grid[SIZE - 1]
counted = {name: sum(row.count(name) for row in top) for name in "wWl"}
print(f"the baked face is the top: {counted} of {SIZE * SIZE}")

with open("content/base/models/water-block.mcvox", "w", newline="\n") as fh:
    fh.write('''# A water block: sparse speckle over one blue.
schema = 1
name   = "base:water"
scale  = 16
size   = [16, 16, 16]
origin = [8, 0, 8]
slice  = "y"

[palette]
"w" = "base:water"
"W" = "base:water_dark"
"l" = "base:water_light"

''')
    for y in range(SIZE):
        fh.write(f'[[layers]]\ny = {y}\ngrid = """\n')
        for row in grid[y]:
            fh.write("".join(row) + "\n")
        fh.write('"""\n\n')

print("wrote content/base/models/water-block.mcvox")

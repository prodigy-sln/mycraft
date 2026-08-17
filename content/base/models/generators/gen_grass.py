"""Generate the grass block's noise fields, and prove they carry no grain.

The structural art here — where the fringe sits, how deep it goes, the shadow
course under the sod — is hand authored. What is generated is only the noise:
which texels take an accent tone. Three hand-written attempts at that noise
produced, in order, a plain band, a vertical stripe and a diagonal lattice,
which is a good argument that a hand is the wrong instrument for the job.

The rule the last two failures share: any speck placement derived from the
previous row by a shift is still correlated, it has only changed direction. A
constant shift per row turns a column stripe into a diagonal one, and a column
histogram cannot see the difference — accents visit every column equally while
marching through them in order. So the placement hashes the coordinate tuple
itself, with no term that relates one axis to another, and the check bins all
four families: rows, columns, and both diagonals.

READ THE PERCENTILE CAREFULLY IF YOU REUSE THIS. The salts below are chosen
*by* the reported test, over a couple of dozen candidates, and the number
printed is therefore a selected minimum-of-maxima rather than a random draw.
It cannot be read as "an 8.5% chance this field is structured" — the search
has already spent some of the evidence it reports. That is harmless for what
it is used for here, which is picking the least structured of N candidates for
generated noise. It is *not* safe as a pass/fail gate over authored content:
for that, score a single unselected draw, or calibrate the alarm against the
distribution of best-of-N rather than against the distribution of one.
"""

BASE_DIRT, DARK_DIRT, LIGHT_DIRT = "d", "D", "m"
GRASS = "g"
GRASS_TONES = ["G", "b", "l", "y"]          # accents, base green excluded
SIZE = 16
DIRT_TOP = 10                                # y = 10 up is fringe or shadow


def mix(*parts):
    """A 64-bit avalanche over the coordinate tuple.

    Deliberately not `x + k*y`: any linear combination of the axes leaves a
    diagonal family invariant, which is exactly the artefact this exists to
    avoid.
    """
    h = 0xcbf29ce484222325
    for value in parts:
        h ^= (value + 0x9e3779b97f4a7c15) & 0xFFFFFFFFFFFFFFFF
        h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
        h ^= (h >> 29)
        h = (h * 0xff51afd7ed558ccd) & 0xFFFFFFFFFFFFFFFF
        h ^= (h >> 32)
    return h


def unit(*parts):
    return mix(*parts) / 0xFFFFFFFFFFFFFFFF


DIRT_SALT = [0x51ED]        # rebound by the search below
BOTTOM_SALT = [0x51ED]


def dirt_tone(x, y, z, salt=None):
    """Earth with roughly a third of its texels accented."""
    r = unit(x, y, z, DIRT_SALT[0] if salt is None else salt)
    if r < 0.66:
        return BASE_DIRT
    if r < 0.85:
        return DARK_DIRT
    return LIGHT_DIRT


def is_corner(x, z):
    return x in (0, SIZE - 1) and z in (0, SIZE - 1)


def dirt_layer(y):
    """A dirt course: hashed perimeter, plain interior nobody ever sees."""
    rows = []
    for z in range(SIZE):
        row = []
        for x in range(SIZE):
            on_edge = x in (0, SIZE - 1) or z in (0, SIZE - 1)
            if not on_edge:
                row.append(BASE_DIRT)
            elif is_corner(x, z):
                row.append(BASE_DIRT)       # corners equal => side wrap is zero
            else:
                row.append(dirt_tone(x, y, z))
        rows.append("".join(row))
    return rows


def bottom_layer():
    """The underside, seen whole, so its own edges have to wrap.

    Last row repeats the first and last column repeats the first, which makes
    the wrap difference exactly zero without flattening anything.
    """
    rows = [[BASE_DIRT] * SIZE for _ in range(SIZE)]
    for z in range(SIZE - 1):
        for x in range(SIZE - 1):
            rows[z][x] = dirt_tone(x, 0, z, BOTTOM_SALT[0])
    for z in range(SIZE - 1):
        rows[z][SIZE - 1] = rows[z][0]
    rows[SIZE - 1] = list(rows[0])
    return ["".join(r) for r in rows]


def top_layer():
    """The growth, seen whole. Dense, five tones, clumped in three shapes.

    Clumps are grown from seeds rather than stamped: a seed picks an
    orientation from the hash, so horizontal pairs, vertical pairs and
    L-shapes appear in roughly equal numbers. One orientation used for all of
    them gives the field a woven grain.
    """
    grid = [[GRASS] * SIZE for _ in range(SIZE)]

    # Bare patches: several small ones rather than two large. A large bare
    # patch is a low-frequency feature, and low-frequency features are what
    # the eye locks onto across a repeat.
    bare = set()
    for i in range(4):
        bx = mix(i, 0xBA5E) % (SIZE - 3)
        bz = mix(i, 0xC0DE) % (SIZE - 3)
        for dx in range(3):
            for dz in range(3):
                bare.add((bx + dx, bz + dz))

    for z in range(SIZE):
        for x in range(SIZE):
            if (x, z) in bare:
                continue
            r = unit(x, z, 0x6A55)
            if r < 0.42:
                continue                     # left as the base green
            grid[z][x] = GRASS_TONES[mix(x, z, 0x70E5) % len(GRASS_TONES)]

    # Clumps: equal thirds of horizontal, vertical and L.
    for i in range(15):
        x = 1 + mix(i, 0xC1) % (SIZE - 3)
        z = 1 + mix(i, 0xC2) % (SIZE - 3)
        if (x, z) in bare:
            continue
        tone = GRASS_TONES[mix(i, 0xC3) % len(GRASS_TONES)]
        shape = i % 3
        cells = [(x, z)]
        if shape == 0:
            cells.append((x + 1, z))
        elif shape == 1:
            cells.append((x, z + 1))
        else:
            cells += [(x + 1, z), (x, z + 1)]
        for cx, cz in cells:
            if 0 < cx < SIZE - 1 and 0 < cz < SIZE - 1:
                grid[cz][cx] = tone

    # The border may not use the two lightest greens: a side texture's
    # vertical wrap pairs this plane against the underside, and a pale green
    # over dark earth exceeds the step the fringe provides to match it.
    for i in range(SIZE):
        for x, z in ((i, 0), (i, SIZE - 1), (0, i), (SIZE - 1, i)):
            if grid[z][x] in ("l", "y"):
                grid[z][x] = "G"
    for z in range(SIZE):                    # last column repeats the first
        grid[z][SIZE - 1] = grid[z][0]
    grid[SIZE - 1] = list(grid[0])           # last row repeats the first
    return ["".join(r) for r in grid]


FAMILIES = {
    "row": lambda x, y: y,
    "col": lambda x, y: x,
    "diag": lambda x, y: (x + y) % SIZE,
    "anti": lambda x, y: (x - y) % SIZE,
}


def worst_ratio(cells, key, total):
    bins = [0] * SIZE
    for x, y in cells:
        bins[key(x, y)] += 1
    mean = total / SIZE
    return max(bins) / mean if mean else 0


def grain_report(name, plane, accents, trials=2000):
    """Bin accents by row, column and both diagonals — against the null.

    A fixed threshold cannot judge this. A side face carries about fifty
    accents across sixteen bins, so a worst bin near 2.4x the mean is simply
    what uniform randomness looks like at that sample size; the top face, with
    five times the accents, lands near 1.3x for the same reason. Comparing the
    two against one number would call the smaller sample grainy and the larger
    one clean when both are equally structureless.

    So each family is scored against its own null: the same number of accents
    scattered uniformly over the same grid, many times over. The percentile is
    the share of random layouts that came out *less* clustered than this one.
    High percentiles are the alarm — 99 means only one random layout in a
    hundred was this streaky, which is what a lattice looks like.
    """
    import random

    rng = random.Random(20260815)
    height = len(plane)
    positions = [(x, y) for y in range(height) for x in range(SIZE)]
    cells = [(x, y) for (x, y) in positions if plane[y][x] in accents]
    total = len(cells)
    out = []
    for label, key in FAMILIES.items():
        observed = worst_ratio(cells, key, total)
        beaten = 0
        for _ in range(trials):
            sample = rng.sample(positions, total)
            if worst_ratio(sample, key, total) < observed:
                beaten += 1
        pct = 100.0 * beaten / trials
        flag = "  <-- clustered" if pct >= 97.5 else ""
        out.append(f"{label} {observed:.2f}x p{pct:4.1f}{flag}")
    return f"  {name:8s} " + "  ".join(out)


def max_percentile(trials=400):
    """The worst percentile over every face and every family."""
    build()
    worst = 0.0
    for name, plane, accents in planes_under_test():
        for line in [grain_report(name, plane, accents, trials=trials)]:
            for token in line.split():
                if token.startswith("p"):
                    try:
                        worst = max(worst, float(token[1:]))
                    except ValueError:
                        pass
    return worst


def build():
    global layers
    layers = {}
    layers[0] = bottom_layer()
    for y in range(1, DIRT_TOP):
        layers[y] = dirt_layer(y)
    layers[15] = top_layer()


def planes_under_test():
    def side(pick):
        return [[pick(layers[y], u) for u in range(SIZE)] for y in range(DIRT_TOP)]
    return [
        ("front", side(lambda L, x: L[0][x]), "Dm"),
        ("back", side(lambda L, x: L[SIZE - 1][x]), "Dm"),
        ("left", side(lambda L, z: L[z][0]), "Dm"),
        ("right", side(lambda L, z: L[z][SIZE - 1]), "Dm"),
        ("bottom", [list(r) for r in layers[0]], "Dm"),
        ("top", [list(r) for r in layers[15]], "Gbly"),
    ]


# Choose the salts by the test rather than by eye. Picking a seed whose output
# happens to have no visible artefact is ordinary practice for procedural
# noise; doing it by looking at renders is how the diagonal survived two
# rounds of inspection in the first place.
layers = {}
best = None
for candidate in range(0x1000, 0x1000 + 24):
    DIRT_SALT[0] = candidate * 0x9E37
    BOTTOM_SALT[0] = candidate * 0x85EB + 0x1234
    score = max_percentile(trials=400)
    if best is None or score < best[0]:
        best = (score, DIRT_SALT[0], BOTTOM_SALT[0])
    if score < 95.0:
        break
DIRT_SALT[0], BOTTOM_SALT[0] = best[1], best[2]
build()
print(f"salts chosen by search: dirt 0x{DIRT_SALT[0]:x} bottom 0x{BOTTOM_SALT[0]:x}"
      f" (worst percentile {best[0]:.1f} over 24 tests)\n")

# The four side textures, assembled the way the tool assembles them, so the
# grain check looks at what actually ships rather than at the source grids.
def side_rows(pick):
    return [[pick(layers[y], u) for u in range(SIZE)] for y in range(DIRT_TOP)]

print("accent grain: worst bin against the mean, and its percentile against the null:")

with open("grass_layers.txt", "w") as fh:
    for y in sorted(layers):
        fh.write(f"@{y}\n")
        for row in layers[y]:
            fh.write(row + "\n")
print("\nwrote grass_layers.txt")

for name, plane, accents in planes_under_test():
    print(grain_report(name, plane, accents))

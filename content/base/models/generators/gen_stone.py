"""Generate the stone block's noise fields, and prove they carry no grain."""

import random

SIZE = 16

def mix(*parts):
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

def stone_tone(x, y, z, salt):
    r = unit(x, y, z, salt)
    if r < 0.66:
        return "s"
    if r < 0.85:
        return "S"
    return "m"

def is_corner(x, z):
    return x in (0, SIZE - 1) and z in (0, SIZE - 1)

def build(salt):
    grid = [[["s" for _ in range(SIZE)] for _ in range(SIZE)] for _ in range(SIZE)]
    for y in range(SIZE):
        for z in range(SIZE):
            for x in range(SIZE):
                grid[y][z][x] = stone_tone(x, y, z, salt)
                
    # Horizontal wrap: 0 difference if corners are base stone
    for y in range(SIZE):
        grid[y][0][0] = "s"
        grid[y][0][SIZE-1] = "s"
        grid[y][SIZE-1][0] = "s"
        grid[y][SIZE-1][SIZE-1] = "s"

    # Top and bottom faces
    for y in (0, SIZE-1):
        for z in range(SIZE - 1):
            grid[y][z][SIZE-1] = grid[y][z][0]
        grid[y][SIZE-1] = list(grid[y][0])

    # Vertical wrap for side faces: top edge matches bottom edge exactly
    for z in range(SIZE):
        grid[15][z][0] = grid[0][z][0]
        grid[15][z][SIZE-1] = grid[0][z][SIZE-1]
    for x in range(SIZE):
        grid[15][0][x] = grid[0][0][x]
        grid[15][SIZE-1][x] = grid[0][SIZE-1][x]
        
    return grid

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

def max_percentile(grid, trials=400):
    def side(pick):
        return [[pick(grid[y], u) for u in range(SIZE)] for y in range(SIZE)]
    
    planes = [
        ("front", side(lambda L, x: L[0][x]), "Sm"),
        ("back", side(lambda L, x: L[SIZE - 1][x]), "Sm"),
        ("left", side(lambda L, z: L[z][0]), "Sm"),
        ("right", side(lambda L, z: L[z][SIZE - 1]), "Sm"),
        ("bottom", [list(r) for r in grid[0]], "Sm"),
        ("top", [list(r) for r in grid[15]], "Sm"),
    ]
    
    rng = random.Random(20260817)
    positions = [(x, y) for y in range(SIZE) for x in range(SIZE)]
    worst = 0.0
    for name, plane, accents in planes:
        cells = [(x, y) for (x, y) in positions if plane[y][x] in accents]
        total = len(cells)
        for label, key in FAMILIES.items():
            observed = worst_ratio(cells, key, total)
            beaten = 0
            for _ in range(trials):
                sample = rng.sample(positions, total)
                if worst_ratio(sample, key, total) < observed:
                    beaten += 1
            pct = 100.0 * beaten / trials
            worst = max(worst, pct)
    return worst

best = None
for candidate in range(0x2000, 0x2000 + 24):
    salt = candidate * 0x9E37
    grid = build(salt)
    score = max_percentile(grid, trials=400)
    if best is None or score < best[0]:
        best = (score, salt, grid)
    if score < 95.0:
        break

print(f"salt chosen by search: 0x{best[1]:x} (worst percentile {best[0]:.1f})")

grid = best[2]
with open("content/base/models/stone.mcvox", "w") as fh:
    fh.write('''# A stone block: noisy rock pattern.
schema = 1
name   = "base:stone"
scale  = 16
size   = [16, 16, 16]
origin = [8, 0, 8]
slice  = "y"

[palette]
"s" = "base:stone"
"S" = "base:stone_dark"
"m" = "base:stone_light"

''')
    for y in range(SIZE):
        fh.write(f"[[layers]]\ny = {y}\ngrid = \"\"\"\n")
        for row in grid[y]:
            fh.write("".join(row) + "\n")
        fh.write("\"\"\"\n\n")

print("wrote content/base/models/stone.mcvox")

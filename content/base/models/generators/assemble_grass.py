"""Assemble grass-block.mcvox from generated noise plus hand-authored fringe.

The split is deliberate. The fringe, the shadow course and the blades are
shape, and shape is authored: a hash does not know that a sod line wants four
depths or that the deepest blade should be the odd one out. The speckle is
noise, and noise is generated: three hand-written attempts produced a plain
band, a vertical stripe and a diagonal lattice in turn.

Provenance, not a build step. This script ran once and produced the grass model
the repository tracks. That model is the source of truth for the art: nothing in
the build, the gate or the client runs this script, and a contributor never needs
to.

It is not maintained and it does not run as it stands. `MODEL` below is an
absolute path naming one machine's checkout, and `HEAD` reads `grass_head.txt`,
which is not in the repository. Both are left unrepaired deliberately — a script
that runs invites somebody to run it, and the tracked model, not this file, is
what the art is made of.

It is kept for the hand-authored courses below, which are design intent recorded
nowhere else. The blade depths and the sod shadow at `y = 10`..`14` were chosen,
not derived; the model carries the result and not the reasoning, and the
reasoning cannot be recovered from the pixels.
"""

import re

SIZE = 16

# ---- hand-authored courses -------------------------------------------------
# y = 10, the sod's shadow. Broken rather than solid, so it is not its own
# horizontal landmark, but dark under every column a blade reaches into.
# One blade now reaches down into it, at x = 6 of the z = 0 face.
LAYER_10 = """DdDDDDgdDDdDDDdD
DddddddddddddddD
dddddddddddddddD
DddddddddddddddD
DddddddddddddddD
Dddddddddddddddd
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
dddddddddddddddD
Dddddddddddddddd
DddddddddddddddD
DddddddddddddddD
dddddddddddddddD
DddddddddddddddD
DDdDDDDDdDDDdDDD"""

# y = 11, the lone blades. Three, not four, and no two the same depth: the
# deepest features are the landmarks, so they are the ones that must not be
# identical to one another.
LAYER_11 = """DDDDDDgDDDDDDDDD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
gddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DddddddddddddddD
DDDgDDDDDDDDDDDD"""

LAYER_12 = """DgDDDgDDDDgDDgDD
DddddddddddddddD
DddddddddddddddD
gddddddddddddddD
DddddddddddddddD
DddddddddddddddD
Dddddddddddddddg
DddddddddddddddD
DddddddddddddddD
gddddddddddddddD
DddddddddddddddD
DddddddddddddddD
Dddddddddddddddg
DddddddddddddddD
DddddddddddddddD
DDgDDDgDDgDDDgDD"""

LAYER_13 = """ggDgDggDggDgggDg
gdddddddddddddDg
Ddddddddddddddgg
gddddddddddddddg
gdddddddddddddDg
Dddddddddddddddg
gddddddddddddddg
gdddddddddddddDg
Ddddddddddddddgg
gddddddddddddddg
gdddddddddddddDg
Dddddddddddddddg
gddddddddddddddg
gdddddddddddddgg
Ddddddddddddddgg
gDggDgggDgDgggDg"""

LAYER_14 = """ggggDgggggggDggg
gddddddddddddddg
gddddddddddddddg
gddddddddddddddD
gddddddddddddddg
Dddddddddddddddg
gddddddddddddddg
gddddddddddddddg
gddddddddddddddg
gddddddddddddddD
gddddddddddddddg
Dddddddddddddddg
gddddddddddddddg
gddddddddddddddg
gddddddddddddddg
ggDgggggggDggggg"""

HAND = {10: LAYER_10, 11: LAYER_11, 12: LAYER_12, 13: LAYER_13, 14: LAYER_14}

# ---- generated courses -----------------------------------------------------
generated = {}
current = None
for line in open("grass_layers.txt"):
    line = line.rstrip("\n")
    if line.startswith("@"):
        current = int(line[1:])
        generated[current] = []
    elif line:
        generated[current].append(line)

layers = {}
for y in range(SIZE):
    if y in HAND:
        layers[y] = HAND[y].split("\n")
    else:
        layers[y] = list(generated[y])

# The blade that reaches into the shadow course needs dark earth under it, or
# its column has no contrast step to match the top-to-bottom wrap. This is the
# one place generated noise has to yield to hand-authored shape.
row = list(layers[9][0])
row[6] = "D"
layers[9][0] = "".join(row)

for y, rows in layers.items():
    assert len(rows) == SIZE, f"layer {y} has {len(rows)} rows"
    for i, r in enumerate(rows):
        assert len(r) == SIZE, f"layer {y} row {i} is {len(r)} wide: {r}"
    corners = {rows[0][0], rows[0][-1], rows[-1][0], rows[-1][-1]}
    assert len(corners) == 1, f"layer {y} corners disagree: {corners}"

MODEL = r"E:\_PROJEKTE\MyCraft\content\base\models\grass-block.mcvox"
HEAD = open("grass_head.txt", encoding="utf-8").read()
head = HEAD

body = []
titles = {
    0: "y = 0, the underside: earth, seen whole from below",
    10: "y = 10, the sod's shadow — broken, and dark under every blade",
    11: "y = 11, three lone blades, no two the same depth",
    12: "y = 12, about three in ten",
    13: "y = 13, about seven in ten",
    14: "y = 14, solid, with two dirt texels pushed up",
    15: "y = 15, the growth itself, seen whole from above",
}
for y in range(SIZE):
    title = titles.get(y, f"y = {y}")
    body.append(f'[[layers]]                # {title}\ny = {y}\ngrid = """\n'
                + "\n".join(layers[y]) + '\n"""\n')

open(MODEL, "w", encoding="utf-8", newline="\n").write(head + "\n".join(body))
print("assembled grass-block.mcvox")

-- The base game's water block.
--
-- The only shipped block that states more than its collision. `solid = false`
-- means one thing and only one: a player falls through it. What water looks
-- like is `drawn` and `occludes`, and what a swing can find is `targetable`,
-- because a block that is seen, a block that hides what is behind it, a block
-- that stops you and a block you can aim at are four different claims.
--
-- `drawn = true` is what puts water on screen. `occludes = false` is what keeps
-- it from hiding the lakebed under it, so the sea reads as water rather than as
-- a lid — and the engine's own rule that a block draws no face against its own
-- kind is what keeps a body of it from being a stack of sheets. Together they
-- are why the sea has a surface and no interior.
--
-- `targetable = true` says a swing can reach water at all; `breakable = false`
-- is what then refuses it. Aiming and yielding are separate on purpose: the
-- refusal is only reachable because the swing arrives.
--
-- `replaceable = true` lets a placement build straight through it rather than
-- requiring it to be broken first, and `breaks_into` is left unstated, so water
-- keeps the documented default
--
-- `swimmable`, `move_resistance` and `swim_ascent` are what make this a medium
-- rather than an absence, and they are three separate claims: being able to
-- hold yourself up in something, being slowed by it, and how hard it lifts you
-- while you hold jump are different properties, and a block may declare any of
-- them without the others. Together they are why a player sinks slowly into the
-- sea, rises by holding jump, and swims more slowly than it walks.
--
-- **All three are read once per tick, and the tick is 60 Hz.** The rates they
-- produce are stated absolutely rather than left to be derived from them: a
-- swimmer asking for nothing sinks `0.9667` blocks in the first second and
-- approaches one block a second thereafter; holding jump raises it
-- `(3.5 - 0.5) / (1 + 0.5)` = `2.0` blocks a second; a walk at full deflection
-- carries it `4.5 / (1 + 0.5)` = `3.0` blocks a second, two thirds of its speed
-- on land. The `0.5` is what gravity takes in one tick and the `4.5` is the
-- walk — both engine constants this declaration cannot reach.
--
-- **The resistance ties the sink to the swim; only the ascent separates the
-- rise from either.** Their ratio `(swim_ascent - 0.5) / 4.5` is free of the
-- resistance entirely — `0.67` here, so the sea climbs a little slower than it
-- carries. That is the whole of what an ascent buys, and it is why the sink was
-- repaired by dropping the resistance rather than by pushing the ascent: at the
-- `1.6` this block used to declare, sinking through the deepest part of the sea
-- took six and a half seconds and a swimmer moved at a little over a third of a
-- walk, which is the complaint this declaration answers.
--
-- **These three numbers are guarded absolutely, and that is a repair rather
-- than a standing property.** Every sea scenario used to read the resistance
-- back out of the registry and state its bound as arithmetic over it, which is
-- what stopped them caging a number they were supposed to leave free — and the
-- cost was that they moved with it and could not see it change. Measured:
-- moving it from `1.6` to `2.0` reddened `terrain_goldens` and nothing else,
-- 701 of 702 still green, because the scripted walk wades through this sea. So
-- a change that moved the number and re-minted the goldens in the same breath
-- was a change nothing reported at all. What closes that now is a second's sink
-- and a second's rise asserted in absolute blocks over absolute ticks, reading
-- none of these three values back.
--
-- **Both halves of that were then measured, workspace-wide, and the ascent's
-- half is the one that matters.** Moving the resistance to `0.6` reddens **five**
-- — the declaration, the sink, the rise, the walk, and the goldens. Moving the
-- ascent to `4.0` reddens **three** — the declaration, the rise, and the
-- surfacing ceiling — **and the golden frames stay green**, because the scripted
-- walk never holds jump under water at the three ticks they are shot at. So the
-- guard that used to be the only one is *blind to this field*: an ascent shipped
-- without a scenario stating its rise in blocks would have been guarded by
-- nothing whatsoever. Each value also has its own sharp witness — the sink moves
-- only with the resistance, the surfacing ceiling only with the ascent.
--
-- **`opacity = 0.5` is what makes the sea see-through, and it is a chosen
-- number rather than a derived one.** Nothing about this world computes a half:
-- it is the degree somebody picked for the sea to read as water — half the light
-- reaching a surface of it goes on through, so the lakebed shows and the sea
-- still reads as a body rather than as a pane. What the number *is* is
-- **binding**: FR-4.3-S1 states `0.5` for a camera standing inside a translucent
-- cell, so the scenario and this line have to agree and changing either one
-- alone reddens a test. Binding is not derived, and the difference matters —
-- deriving the content from a number a spec happens to state, and then calling
-- the content derived, is circular.
--
-- `occludes = false` above was already saying the sea does not hide the lakebed,
-- and it is what a mesher reads. This says how much of the light a *drawn* face
-- of it stops, which is what a fragment reads, and the loader refuses the two
-- together at `occludes = true` precisely because they would then disagree.
return {
	name = "base:water",
	texture = "base:water",
	solid = false,
	breakable = false,
	replaceable = true,
	drawn = true,
	occludes = false,
	targetable = true,
	swimmable = true,
	move_resistance = 0.5,
	swim_ascent = 3.5,
	opacity = 0.5,
}

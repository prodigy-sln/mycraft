-- The all-opaque fixture's water, and the block this whole root is about.
--
-- The shipped sea becomes translucent in phase 3 of SPEC-031. This one does not
-- and may not: the recording beside this root is the oracle's prediction over a
-- world in which *every* drawn block is opaque, and a sea that let light through
-- would make the recording a statement about a different world.
--
-- The shipped water declares `swimmable`, `move_resistance` and `swim_ascent`
-- besides. They are absent here on purpose: nothing under this root runs a
-- simulation, so a swimmer's rates would be three numbers no reading consults
-- and three more that could drift.
--
-- See `stone.luau` for why `drawn` is stated rather than defaulted.
return {
	name = "base:water",
	texture = "base:water",
	solid = false,
	drawn = true,
	occludes = false,
}

-- The all-opaque fixture's stone.
--
-- This root exists so that one world stays all-opaque on both sides of
-- SPEC-031: the tree before a block could declare an opacity, and the tree
-- after. It is a fixture of the test suite and not content anybody plays, so
-- it declares only what the reading it serves depends on.
--
-- `drawn` is stated rather than left to its default at every block here, and
-- that is the point of the file. The recording beside this root is a march that
-- stops at the first *drawn* voxel, so a change to what an unstated `drawn`
-- means would move every value in it — silently, and in a direction that reads
-- as an oracle defect rather than as a moved default.
--
-- **No block under this root may ever state an opacity.** The moment one does,
-- the world stops being the all-opaque world the recording is of, and the
-- scenario the recording serves has lost its premise rather than failed.
return {
	name = "base:stone",
	texture = "base:stone",
	solid = true,
	drawn = true,
}

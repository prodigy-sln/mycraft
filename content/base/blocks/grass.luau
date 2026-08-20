-- The base game's grass block.
--
-- A declaration is a Luau chunk that returns a table. It is code that ran, not
-- a document that was parsed, so a block may compute what it declares — but a
-- shipped declaration says what it means plainly and leaves the computing to
-- content that needs it.
--
-- `texture` is a namespaced key and never a file path: what pixels a key
-- resolves to is the renderer's concern.
--
-- This is the first shipped block to state a texture per facing, and it is the
-- reason the table form exists. Turf on top, bare dirt underneath, and a band of
-- turf over dirt on each of the four sides — six facings and six keys, all of
-- them baked from one model. `down` names `base:dirt` on purpose: the grass
-- block's underside *is* plain dirt, so it is the same image the dirt block
-- draws on all six of its own faces, and no dirt model exists. A second model
-- would be a copy of the same voxels, and the day the two drifted a grass block
-- would sit on ground of a different colour.
--
-- The four sides are four different images rather than one repeated: the model
-- is asymmetric, so `content/base/textures.toml` bakes each side of it under its
-- own key. Naming one key against two facings would be cheaper by a layer and
-- is what `docs/modding/blocks-items.md` shows an example of; it is not what
-- this model is.
return {
	name = "base:grass",
	texture = {
		up = "base:grass_top",
		down = "base:dirt",
		north = "base:grass_side_north",
		south = "base:grass_side_south",
		east = "base:grass_side_east",
		west = "base:grass_side_west",
	},
	solid = true,
}

-- The all-opaque fixture's grass. See `stone.luau` for what this root is for
-- and for why `drawn` is stated rather than defaulted.
--
-- One texture key covers all six facings here where the shipped grass names six.
-- Nothing under this root is ever textured, meshed or drawn — the only question
-- asked of it is which blocks a marched ray meets — so a per-facing declaration
-- would be six names nothing reads.
return {
	name = "base:grass",
	texture = "base:grass",
	solid = true,
	drawn = true,
}

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
return {
	name = "base:water",
	texture = "base:water",
	solid = false,
	breakable = false,
	replaceable = true,
	drawn = true,
	occludes = false,
	targetable = true,
}

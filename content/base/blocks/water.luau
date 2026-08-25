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
-- `swimmable` and `move_resistance` are what make this a medium rather than an
-- absence, and they are separate claims: being able to hold yourself up in
-- something and being slowed by it are different properties, and a block may
-- declare either without the other. Together they are why a player sinks slowly
-- into the sea, rises by holding jump, and swims more slowly than it walks.
--
-- The resistance is the **geometric centre of the window the six sea scenarios
-- admit**, measured against the built simulation rather than chosen: the deepest
-- lakebed must be surfaced within 120 ticks, which caps it at 16.0, and the sink
-- must land inside its budget, which floors it at 0.16. 1.6 stands a factor of
-- ten from each end, so no physics constant or scenario threshold moves the
-- shipped sea out of the window without moving it a long way.
--
-- **Exactly one test guards this number, and it is not one of the swimming
-- ones.** Measured rather than assumed: moving it to `2.0` — another value the
-- window admits — reddens `terrain_goldens` and nothing else, 701 of 702 still
-- green. Every sea scenario reads this value back out of the registry and states
-- its bound as arithmetic over it, which is what stops them caging a number they
-- are supposed to leave free — and the cost of that is that they move with it and
-- cannot see it change. What reports a changed value is the committed golden
-- frames, because the scripted walk wades through this sea. So a change here that
-- re-mints the goldens in the same breath is a change nothing in the suite
-- reports at all.
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
	move_resistance = 1.6,
}

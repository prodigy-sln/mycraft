-- The base game's water block.
--
-- The only shipped block that is neither solid nor an obstacle to building:
-- `solid = false` keeps the mesher from culling against it and keeps a player
-- from standing on it, and `replaceable = true` lets a placement build straight
-- through it rather than requiring it to be broken first.
--
-- `breaks_into` is left unstated, so water keeps the documented default
return {
	name = "base:water",
	texture = "base:water",
	solid = false,
	breakable = false,
	replaceable = true,
}

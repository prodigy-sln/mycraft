-- The base game's water block.
--
-- The only shipped block that is neither solid nor an obstacle to building:
-- `solid = false` keeps the mesher from culling against it and keeps a player
-- from standing on it, and `replaceable = true` lets a placement build straight
-- through it rather than requiring it to be broken first.
--
-- `breakable` and `breaks_into` are left unstated, so water keeps the documented
-- defaults: breakable, leaving the cell empty.
return {
	name = "base:water",
	texture = "base:water",
	solid = false,
	replaceable = true,
}

-- The base game's dirt block.
--
-- A declaration is a Luau chunk that returns a table. It is code that ran, not
-- a document that was parsed, so a block may compute what it declares — but a
-- shipped declaration says what it means plainly and leaves the computing to
-- content that needs it.
--
-- `texture` is a namespaced key and never a file path: what pixels a key
-- resolves to is the renderer's concern. It is declared equal to `name` here
-- because the layer a block draws from is selected by its **name** today.
return {
	name = "base:dirt",
	texture = "base:dirt",
	solid = true,
}

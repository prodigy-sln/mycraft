# What a script may reach

**What this answers:** exactly which global names a chunk can see, which are
gone, and why the environment refuses to be written to. **Who it is for:**
anyone writing Luau who wants to know what is there before reaching for it.

**Block declarations are Luau and run inside this surface** — see
`blocks-items.md` for what a declaration states and `README.md` for writing your
first one. What follows is the whole of what a chunk can reach.

Every chunk is evaluated inside one sandboxed Luau state, closed before any
content runs. The base game has no privileged path into it: this is the whole of
what any content can reach.

## The reachable surface, exactly

The host declares the names a chunk can reach, and the declared set is compared
against what a chunk can *really* reach in both directions — a name reachable
but undeclared is reported, and so is a name declared but gone. That is the
guard against a capability arriving unannounced in a future Luau or `mlua`
release, which a deny list alone cannot see.

**Reachable — 31 names:**

```
_G          _VERSION    assert      bit32       buffer      coroutine
error       getmetatable            integer     ipairs      math
next        pairs       pcall       print       rawequal    rawget
rawlen      rawset      select      setmetatable            string
table       tonumber    tostring    type        typeof      unpack
utf8        vector      xpcall
```

**Denied — 14 names**, removed by the host itself before the sandbox is closed:

```
io          os          package     require     loadstring  load
dofile      loadfile    debug       getfenv     setfenv     collectgarbage
newproxy    gcinfo
```

Closing Luau's own sandbox removes five of those fourteen and leaves the rest
standing, so the host does not rely on which half is which and removes all
fourteen. Four of the denials are worth understanding rather than memorising:

- **`getfenv` / `setfenv`** are the escape pair. Either one hands a chunk
  another chunk's environment, which is the isolation everything else here
  rests on.
- **`collectgarbage`** drives the collector, which is a memory side channel
  and a way to move the ground under everybody else's allocation.
- **`newproxy`** is a `__gc` vector: a finaliser is script the host never asked
  to run, at a moment the host did not choose.
- **`gcinfo` is denied for determinism**, not for secrecy. It reports the heap
  size, so a script branching on it is a function of the collector's state
  rather than of its own inputs. World generation is the caller that cannot
  survive that: a generator branching on heap size returns different terrain
  for one seed, which loses a world rather than leaking a number. It is
  `collectgarbage`'s objection from the other direction — one reads the
  collector, one drives it.

`require` is **absent rather than confined**. There is no mod-directory concept
to confine it to, so a mod is one chunk; multi-file mods and a confined
`require` arrive together, with path traversal out of a mod's own directory
treated as the security question it is.

**`print` reaches the host, and only the host.** Luau's own `print` writes to C
`stdout` — a different buffer, outside every log the host controls — so the
host installs its own **before** the sandbox is closed. The ordering is the
rule: installing or removing a global after the sandbox is closed silently
succeeds and changes nothing, because a write to a global then lands in a child
table and removes nothing. Everything a chunk prints is collected by the host,
in order, as **unbounded text your mod chose**; whoever routes it to a log
inherits both its content and its length.

## The environment is frozen, so you write locals

Each chunk is evaluated against a fresh table of its own, frozen, whose
metatable reads through to the shared globals. Three consequences:

- **A global assignment is an error**, not a silently ignored write.
  `heat = 0` at the top of a chunk aborts that chunk at the assignment, naming
  it. Write `local heat = 0`.
- **`rawset` does not help.** `rawset(_G, "smuggled", 1)` is refused where it
  lands, because the table is frozen rather than guarded by a metamethod a
  write can decline to consult.
- **Shared library tables are frozen too.** Assigning to `string.format`, or
  hanging a new `__index` on the shared string metatable, is refused; the next
  chunk observes the original behaviour.

The environment is per chunk rather than shared, and that is containment rather
than tidiness: with one shared environment, a mod writing `print = function()
end` silences every other mod on the server. With its own environment, that
write is refused *and* would have been private if it had not been.

The refusal happens at the assignment, so a chunk that replaces a global and
then calls it never reaches the call at all. If you want to observe the refusal
without losing the rest of the chunk, wrap it: `pcall(function() print = f end)`
returns `false`, and the `print` that follows still reaches the host.

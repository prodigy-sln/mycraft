# Working in this repository

How this tree is actually worked, as built. Not rules about evidence — those are
`testing.md`'s — and not standards, which are constitutional and live in
`standards/`. What is here is the handful of facts about the *environment* that have
cost real time to rediscover.

The one thing to know before the rest: **this repository is normally worked by
several agents at once, in one working tree, sharing one git index.** Most of what
follows is a consequence of that.

## In a shared tree, an unexpected failure is a question about who else is working before it is a question about what is broken

And the operational half, which is what somebody needs at 3am: **never remove a live
`.git/index.lock`. Wait, then retry.**

Four measured instances, each a foreign action wearing the costume of a local defect:

| What was seen | What it actually was |
|---|---|
| A test failing, then passing on re-run — a flake | Another agent's mutation live in the tree during the run |
| The gate red on a stage that had never fired | Another agent's untracked files inside a directory a guard scans |
| `fatal: Unable to create '.git/index.lock': File exists`, whose own text invites removing it | Another agent mid-commit; the lock cleared in one second |
| Shipped content declaring `base:stone` non-solid — a content regression | Somebody's hand-run of a manual acceptance check, left in the tree |

The third is the dangerous one, because **the error message recommends the
destructive move**: removing a live lock corrupts somebody else's in-flight commit
rather than recovering your own.

The fourth wears a different costume from the first three and that is why it is
listed separately. The others are a foreign *action* mistaken for a defect; this is a
foreign *acceptance run* mistaken for a regression — and the edit was one the
documentation asks a person to make, so it looks exactly like the docs being followed
correctly. The response that works: revert by hand, confirm with `git diff
--exit-code` over the affected tree, and **tell the owner rather than leaving the
revert silent.**

**Announce a mutation window before you open one, and say when it closes.** A window
nobody knows about turns every concurrent run into a mystery, and the mystery is
expensive in both directions — the other agent chases a defect that is not theirs,
and you get a reading that is not yours.

### Remove the collision before you coordinate around it: run acceptance in its own root

**The rule first, because the entry below is the fallback and this is the answer.** A
hot-reload acceptance check mutates shipped content, and it does not have to. The client
resolves its content root *and* its save relative to the directory it was started in —
`main.rs` states that as a deliberate choice, and `shipped_directory()` is a relative
path checked against the working directory. So:

```text
mkdir -p playground/content && cp -r content/base playground/content/base
cd playground && cargo run --manifest-path ../Cargo.toml -p mc-client
```

That session watches `playground/content/base`, saves into `playground/saves`, and
mutates nothing any verification run reads. **`docs/modding/hot-reload.md` leads with
this as the supported way to test a content edit**, not as a workaround.

The announced window below is for the case where the shipped tree genuinely must be the
subject — re-shooting a golden set, say. **Remove the collision first; announce it only
when you cannot.**

### The hot-reload manual acceptance check is a mutation window, and must be announced as one

The fourth instance above is an acceptance run whose edit was *left* in the tree. The
sharper version is one **in flight during a verification run**, and it happened here:

> **A manual acceptance check of hot reload is, by construction, a mutation of shipped
> content in a shared tree — and the gate cannot tell it from a regression.**

What the gate reported was a golden mismatch with **706 743 of 921 600 pixels past
tolerance**, plus a control assertion failing in a second suite. Both read as a
catastrophic rendering regression, with total confidence, because the content the gate
read genuinely *was* different: non-solid stone re-meshes the whole terrain. Nothing in
either failure text could distinguish it.

**The instruments that did:** both tests passing in isolation, `git diff` over the
content tree coming back clean at both ends, and — the decisive one — **file mtimes
inside the window the gate ran in.** A file edited and reverted leaves no diff and does
leave a timestamp.

So: announce the window, and do not run the gate inside somebody else's.

### A gate reading is a statement about a tree, not about a commit

The gate reads the **working tree**. In a shared tree that is routinely not what HEAD
contains: an exit-0 run has passed here over another agent's uncommitted test file, so
the tree was green and HEAD was not. Both readings are true and only one of them is a
merge condition.

So a report of a green gate says **which commit it was taken at and whether the tree was
clean at the time**, and whoever merges re-runs it at HEAD regardless of who reported
what. `git status --short` beside the exit code is the whole of the discipline.

**And a dirty path is not necessarily an edit.** A hand-reverted mutation leaves a stale
stat entry, so `git status` shows ` M` on a file whose diff is empty. `git diff
--exit-code -- <path>` is what tells the two apart, and it is the check to run before
reporting a tree as dirty — twice in this spec a file was reported as modified when it
was byte-identical to HEAD.

### Another agent's red is a hypothesis with a timestamp — and so is your own

Re-check a red against the tree before letting it gate you. One `cargo fmt --check` and
one `grep` is cheaper than a held gate and cheaper than the message asking whose it is.

**The sharper form, because the ordinary one misses the case that actually bites:** a red
*you observed first-hand* ages exactly as fast as one somebody reported to you. First-hand
observation is what makes an ageing fact feel exempt from being re-checked, and it is not.
A gate was held here on two reds that had been fixed in a commit the holder's own work was
already sitting on top of.

## `git add <paths>` is not explicit-path staging when the index is shared

`git commit` commits **the index**, so anything a concurrent agent staged rides along
even when you named your own paths to `git add`. The safe form is:

```
git commit -- <paths>
```

The implied `--only` commits exactly those paths and leaves another agent's staged
file in the index untouched. Measured in a scratch repository rather than assumed:
with `theirs.txt` staged by someone else, `git commit -- mine.txt` committed one file
and left `theirs.txt` staged.

`standards/global/git-workflow.md` §2 bans `git add -A` and `git add .`. Both agents
in the instance that produced this rule obeyed that ban, and a file still crossed the
boundary — the hazard is one step past what the ban covers.

**Recovery, when something has already ridden along.** The instinct is to unpick it
and that is usually wrong: a feature branch squash-merges, so attribution to a commit
*on the branch* is immaterial to `main`, and unpicking means reaching into another
agent's index for no durable gain. Confirm the content survived — `git show --stat`
reading `| 0` for a moved path is what establishes that a rename carried no content —
then commit on top.

**Verify a git incantation somewhere harmless before using it on a shared tree.** The
scratch-repository check above cost a minute and is cheaper than any of the ways that
goes wrong.

## When a file approaches its size limit, read its own header for the seam

`standards/global/code-quality.md` §2 says to split by responsibility when a limit is
exceeded. This is how that responsibility gets *found* here:

> **A header written to explain the file to a reader has usually already drawn the
> line the limit is about to force.**

Twice in one spec — `session.rs` and `app.rs` — a file crossed 500 non-blank lines and
the boundary was already named in its own header before anyone went looking, which is
why both splits came out clean rather than arbitrary. Each became a directory with the
crossing responsibility in a **child** module, because the extracted code writes the
parent type's own fields and a sibling cannot see them.

The failure mode this avoids is worth naming: a boundary invented to satisfy a count
is a boundary nobody will respect, and the usual next move — shaving a file header to
buy four lines — deletes the reason the file exists in order to satisfy a limit that
was never about the header.

### A split becomes `foo/mod.rs`, and that is not up for decision at the moment of a split

Rust offers two layouts for the same module tree — `foo/mod.rs`, or `foo.rs` beside a
`foo/` directory — and **this repository uses the first one everywhere it has a
choice.** Measure it rather than taking it on trust:

```bash
find crates tools -name mod.rs | wc -l                      # 48 at the time of writing, 33 under a src/
for d in $(find crates tools -type d -not -path '*/target/*'); do \
  b=$(basename "$d"); p=$(dirname "$d"); \
  [ -f "$p/$b.rs" ] && echo "$p/$b.rs + $d/"; done            # the sibling pairs
```

The second command reports exactly two, and **neither is a choice**:
`crates/mc-render/build.rs`, where Cargo requires a build script to sit, and
`crates/mc-client/tests/support/reload_watch.rs`, which is test support. There are
none at all in a production `src/`.

**Why this is written down rather than left to taste.** A split done to get under a
line cap once landed as `luau_declaration.rs` + `luau_declaration/`, which was one
style deviation in a tree that otherwise agrees with itself, and it was ruled back.
The reasoning generalises: **both layouts give the identical module tree, so the
load-bearing part of the split argues for neither** — which leaves consistency as the
only live consideration, and consistency is not something a file wins by being the
newest. Moving this repository to the sibling form is a call somebody takes
deliberately, on its own, and never one made in passing while getting a file under a
limit.

## A `pub` item on a `pub` type is invisible to dead-code lints

**At a crate's public boundary, "does anything actually call this?" is a question only
a reader can answer.** No dead-code lint reports a `pub` item on a `pub` type, however
few callers it has — so *"a test finds it handy"* survives unchallenged by
construction.

Two instances:

- A `pub` accessor handing a test the list of sections a discarded re-mesh batch would
  have covered. Clippy at `-D warnings` was clean with it and without. Its only caller
  was a fixture, and what it made easy to write was an assertion on *which* sections
  were discarded in place of one on their being meshed again — the weaker of the two by
  exactly the margin that matters.
- `ContentView::is_solid` in `mc-client`: a `pub` accessor with **no production
  caller** and two test files asserting against it. Neither a lint nor coverage flagged
  it — **coverage saw it exercised.**

The actionable form: when a `pub` item's callers are all tests, the question is whether
removing it puts a weaker assertion out of reach. Prose asking an author to prefer the
stronger assertion is a request to remember; an absent method is not.

**And where a visibility is doing work, say so at the declaration.** One method here is
`pub(crate)` with exactly one caller, and that is what makes forgetting the call fail
the build *twice* — once for the unused binding and once because dropping the call
orphans the method's only consumer. Widening it to `pub` keeps the first diagnostic and
silently loses the second. A reader about to widen a visibility is looking at the
declaration, not at a document.

## A green suite is no evidence about a lint, and a green clippy is no evidence about rustdoc

Two instruments, two blind spots, both measured here.

**The lint.** A nesting-threshold defect survived 697 passing tests and two rounds of
falsification. A suite and a lint answer different questions, and the only instrument
that can report the second is the gate — so anything only the gate can see accumulates
silently across any window in which the tree does not compile. Run
`cargo clippy --workspace --all-targets --all-features -- -D warnings` directly rather
than waiting for a phase boundary. Checking at a lower severity asks a different
question, and without `-D warnings` cargo attributes the diagnostic to the first binary
and marks the rest `(1 duplicate)` — which means *this same diagnostic, repeated*, not
*a pre-existing one lives elsewhere*.

**The rustdoc.** `cargo fmt --check` clean, clippy at exit 0, 1 177 tests passing, and
the gate's rustdoc stage red. The cause:

> **A module carrying both an outer `///` doc on its `mod` declaration and an inner
> `//!` header has the whole block's links resolved in the *parent's* scope.**

So a bare local name in the header — ``[`LocalType::method`]`` — is unresolvable, while
the identical link inside an item's own doc resolves fine. Use a fully-qualified path,
or plain backticks, in a module header. Two things make this expensive to rediscover:
**no instrument an edit loop runs can see it**, only
`cargo doc --workspace --no-deps` under
`RUSTDOCFLAGS='-D warnings -D rustdoc::broken_intra_doc_links'`; and **the error text
points at the header rather than at the `mod` line**, so the natural reading is that
the link is wrong rather than that the scope is somewhere else.

**It recurred within the hour of being written down**, in a header being edited for an
unrelated reason. The instinct to write ``[`LocalItem`]`` in a module header is stronger
than the memory of why it does not work there — which is the argument for this page
existing rather than for anybody remembering.

## A markdown-only change is not gate-neutral

The natural assumption is that a commit touching nothing but `.md` cannot fail the gate,
and it is *nearly* true, which is what makes it worth writing down. Every stage that
parses Rust ignores markdown, and the size stage measures `*.rs` only
(`sdd-gate.ps1`, `$SizeRoots` walk). **The secrets stage does not.** It runs
`gitleaks dir .`, and `dir` scans the **working tree** — every file in it, whatever the
extension, tracked or not.

So a documentation change can fail the gate, and the realistic way is not a real
credential. It is a **long high-entropy literal**: a sha256 digest recorded as evidence,
a base64 fixture, an example token in a modding page. A 64-character hex string is the
shape an entropy rule is built to notice, and whether a given one trips depends on the
rule set rather than on the author's intent.

Two consequences:

- **Run the gate for a docs-only change too**, or at minimum
  `gitleaks dir <path> --no-banner --redact --exit-code 1` over what you touched. The
  surprising case is exactly the one where somebody skips the gate believing markdown
  cannot fail it.
- **gitleaks is optional in the gate**, so a missing binary is a warning rather than a
  failure (`Test-ToolPresent 'gitleaks' … -Optional`). A docs-only change can therefore
  pass on a machine without it and fail on one with it — the two readings are not the
  same statement, which is the point [the section on a gate reading being about a tree
  rather than a commit](#a-gate-reading-is-a-statement-about-a-tree-not-about-a-commit)
  makes from the other side.

## Lifting a limitation costs one grep per place it was stated

A spec that *adds* a surface is documented by the obligation in `CLAUDE.md`'s Key
Principle 3 — the author knows what they built and writes it down. A spec that
**lifts a stated limitation** has no such prompt, and the work is the mirror image:
the new surface documents itself, while every passage that recorded the old
limitation goes silently false. Nobody is reading those passages, because the person
who lifted the limitation was never in them.

**So the closing step of such a spec is a grep for the limitation's own words**, not
a re-read of the pages the spec touched. A limitation deliberately documented in
several places — which is good practice, and the reason it was in several places — is
one that costs several edits to lift.

**The instance this comes from.** SPEC-019 lifted two stated limitations at once: that
real art had not landed, and that a face's layer was selected by the block's *name*
rather than by its declaration. Validation found **six findings over two passes, every
one stale documentation and not one a code defect** — four in the first pass, two more
in the second that the first had read past. Fixing those six by name and then grepping
for the limitations' own words turned up **ten further stale passages neither pass had
touched**, in a routing table, a first-block tutorial, a format-summary table, a
directory `CLAUDE.md`, a planning document and four cross-reference sentences. The new
surface had been documented correctly the whole time. Nobody asked *what did this
change make untrue?*

Three shapes are worth knowing because they are where the stale copies hide:

- **An ADR whose Consequences name another document as their record.** ADR-024 said
  the `texture` limitation "is stated in `modding/hot-reload.md` rather than papered
  over" — so lifting it needed an edit *there* and an edit *in the ADR*, and the ADR
  is precisely the file nobody re-reads. A pointer between two documents is a second
  place the fact lives, not a way of having it in one place.
- **A tutorial's worked example, and a routing table's summary of a page.** The
  comment inside `modding/README.md`'s first block, and `docs/INDEX.md`'s one-line
  digest of `hot-reload.md`, both restated the limitation in their own words. Neither
  contains the phrase the as-built pages use, and `INDEX.md`'s rows are long enough
  that a stale clause reads as prose rather than as a claim.
- **A synonym for the limitation, on a page already corrected for this very class.**
  SPEC-020 lifted the refusal of a save whose blocks *behave* differently, and found
  the old behaviour recorded across fourteen files as a **prompt that has never
  existed** — a refused launch naming a flag, described as the game asking. Five of
  those lines were shipped `mc-world` doc comments *arguing from* the prompt. The worst
  was player-facing and false about the shipped program, `docs/user/gameplay.md`'s "the
  game asks before opening a save whose blocks behave differently", on a page corrected
  for this same class two commits earlier. It survived that correction because the
  correction grepped the limitation's own phrase and this line used a synonym for it.
  **So the grep is over the limitation's vocabulary, not its wording** — every word the
  old behaviour was ever described with, including the ones no as-built page uses, and
  including the descriptions that were never accurate in the first place. A wrong
  account of a limitation does not become findable by lifting the limitation.

The general falsifiability form of this — that a green suite is no evidence about a
document — is in `technical/testing.md`. This section is about the habit that catches
it: **when a spec removes a "today" or a "not yet", grep for the words it just made
false before calling the spec done.**

## A living page carries the command; a dated observation carries the number

The same drift arrives a second way, and it is quieter because nothing was ever
false when it was written.

> **The durable form of a measurement is the command that reproduces it, not the
> number it produced.**

A recorded number has to be maintained, and **a number nobody maintains becomes a
confident lie** — it goes on reading as a measurement long after it stopped being
one. One spec met this at four levels inside a single day: a scenario count restated
in prose and wrong twice over, a routing-table summary naming a count the page it
described owned, an arithmetic total, and the digest table the rule was drawn from.
None of the four was a careless entry. Each was correct when it was typed.

**So the two kinds of document take numbers differently, and the difference is the
rule:**

- **`docs/` is maintained, so it carries the means of reproducing a figure** — the
  command, the derivation, the file the number can be read out of. Where a figure
  itself is load-bearing enough to state, state the command beside it, so the next
  reader can find out in a minute whether it still holds. Several sections on this
  page do exactly that.
- **A spec folder is archived and pruned, so it is a *dated observation* and may
  carry bare numbers freely.** Nobody is obliged to keep it current; that is what
  makes its numbers honest.

Applied concretely: the twelve sha256 digests taken while establishing that the
shipped art reproduces from the checked-in voxel models **stayed in the spec folder
and were deliberately not moved into `docs/`.** Their permanent successor is a claim
a reader can re-run instead of compare against — the build's byte-identical rebuild
plus the index fold (`modding/voxel-models.md`). A digest table in a maintained page
is a second copy needing an update on every art change, and **the copy that stops
being updated is the one a reader trusts.**

**The one legitimate exception is a document that cannot change.**
`technical/licensing.md` records hashes and byte counts for `LICENSE-MIT` and
`LICENSE-APACHE` precisely because those texts are fixed upstream: there is no
future edit for the figure to fall behind. That is the test to apply — not "is this
number interesting" but **"what would change it, and would that change also update
this line?"**

One corollary that has cost an edit here: **a routing summary describes what a page
answers, never how many answers it has.** A count in `INDEX.md` is a second copy of
something the page itself owns, and it is fixed by deleting the number rather than by
correcting it. The related trap on the other side — that an instruction to update a
figure is itself a claim, measured before it is obeyed — is in `technical/testing.md`
with the instance behind it.

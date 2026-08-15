# Licensing — the texts, their provenance, and the drift check

The workspace declares `license = "MIT OR Apache-2.0"` and now ships both
texts at the repository root. This file records what a reader needs before
touching either of them: where the bytes came from, what deliberately is
*not* filled in, and what the automated check does and does not cover.

The declaration is the one claim a third party relies on before using or
contributing anything, so it is checked against the tree on every gate run
rather than trusted.

## What ships

| File | Bytes | sha256 |
|---|---|---|
| `LICENSE-APACHE` | 11358 | `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30` |
| `LICENSE-MIT` | 1074 | `20c57d18f12e6255af73f9ca23612367d55e09cbce7cfc14081247349e087f6c` |

Both are pure ASCII with LF endings and no CR byte anywhere.
`.gitattributes` carries `LICENSE-* text eol=lf` so a Windows checkout
hands a redistributor the same bytes a Linux one does — a redistributor
gets the working tree, and a licence is the one file they are entitled to
receive unaltered.

The copyright line is `Copyright (c) 2026 prodigy.solutions`, written in
**one place, `LICENSE-MIT`**. The holder is the company, not the individual:
the README byline, the repository owner and the Linear team all say so, and
only commit authorship pointed the other way — authorship of a commit is not
ownership of the work. Nothing derives from the string.

## Provenance — read this before replacing either file

Byte-authenticity has no offline mechanical oracle. A hash computed from the
file we just wrote is circular, so the provenance below was established once,
by hand, and is recorded here because the next person to touch these files
cannot re-derive it from the tree.

- **`LICENSE-APACHE` is byte-for-byte the apache.org text.** The hash above
  was cross-checked against every `LICENSE-APACHE` in the local cargo
  registry cache: **15 crates ship this exact byte sequence.** Two common
  ecosystem variants differ, and **the 9723-byte variant is short enough
  that it would fail a scenario** — it is not merely a different wrapping.
  If a replacement is ever needed, take it from apache.org and check the
  hash; do not copy one out of a dependency without checking which variant
  it is.
- **`LICENSE-MIT` matches SPDX's own `MIT.json` word for word** — 168 words,
  exact modulo line wrapping and the two substituted fields (year, holder).

Both were verified against every structural anchor *before* being placed,
which is why the checks went green on the first run rather than by
iteration. That ordering is what makes the green meaningful: no assertion
was bent to fit a text.

## `LICENSE-APACHE`'s appendix stays unfilled — and it is a trap

Line 190 reads, verbatim:

```
   Copyright [yyyy] [name of copyright owner]
```

**This is the canonical published form of the Apache License 2.0 appendix**
and must stay exactly as it is. The appendix is instructions *for applying*
the licence, not a notice about this work; filling it in would make the file
no longer the licence.

The consequence is sharp enough to be worth stating twice: that line carries
copyright *shape* **and** an unfilled placeholder marker at the same time, so
**a placeholder detector shared across both texts reports a violation against
a byte-perfect file.** The detector this project ships is therefore scoped to
the MIT reading alone. Anyone widening it will meet this again — the correct
response is to keep the scope narrow, never to edit the licence.

`LICENSE-APACHE` carries no holder for the same reason, which is why the
holder's name appears in one file rather than two.

## How much SPDX the check understands

A flat list of bare identifiers joined by **one** operator kind (` OR ` or
` AND `), mapped through a **closed** table:

| Identifier | File |
|---|---|
| `MIT` | `LICENSE-MIT` |
| `Apache-2.0` | `LICENSE-APACHE` |

Both operator kinds are accepted because they yield the same required set,
and refusing one would be a false alarm on a declaration whose required
texts had not changed — a drift check that cries wolf gets suppressed.

**Everything else is refused by name rather than resolved to a guessed
filename**: identifiers outside the table, `WITH` exceptions, parenthesised
sub-expressions, the `+` "or later" suffix, `LicenseRef-` identifiers, mixed
operator kinds. A guessed filename lets a licence change through with a
plausible-looking file; a refusal forces a human to decide what text the new
licence needs. Not-understanding is itself a failure, which is also what
closes the vacuity hole parsing would otherwise open.

**`MIT+` is the sharpest case and the reason `+` is refused rather than
stripped.** Naively reduced to `MIT`, it finds `LICENSE-MIT`, reports
success, and ships no text for the "or later" the project actually declared.

The narrowness is deliberate and is not a stub to be finished. Widening it
means adding a real SPDX parser — operator precedence, nested parentheses,
identifier validation against the official list — which needs a dependency
or an embedded copy of the list and buys nothing while the table is closed.

## The check refuses rather than passing vacuously

Three properties are universally quantified over collections that can be
empty, and each has an explicit guard, because "every declared licence has a
text", "every licence link resolves" and "no file contains a CR byte" are all
trivially true of nothing:

- an empty licence expression, or metadata carrying no declaration at all,
  is a refusal — not an empty required set;
- a README with zero licence links is a refusal naming the zero;
- a byte scan that read zero licence files is a refusal naming the zero, and
  a passing byte scan **names the set it graded** so it cannot be silent
  about having graded nothing.

Verdicts are asserted as an enumerated value, never as the absence of a
finding. That distinction is load-bearing here — see
`technical/testing.md`, "Exact verdict beats an absence assertion".

## Where the check lives, and why there

`crates/mc-world/tests/`, in four files plus shared fixtures. Three
constraints decided the placement and are recorded so they are not
rediscovered:

- **A root `tests/` directory is impossible.** The workspace manifest is a
  pure virtual manifest with no `[package]` section, so a root `tests/` dir
  has no target and `cargo nextest run --workspace` would never see it.
- **`mc-core` is excluded on a hard constraint.** `dependency_graph.rs`
  asserts `toml` is absent from `mc-core`'s resolved closure and treats
  dev-dependency edges as real, so adding any parser there turns an existing
  invariant red.
- **`mc-world/tests/` wins on precedent** — it already hosts the repo-wide
  scans that walk every crate's `src/`, and `tests/common/mod.rs` already
  exports `repository_root()` and `TestResult`.

Recorded with a trigger: **a third unrelated repo-hygiene invariant justifies
a dedicated crate.** Two do not.

The declaration is read through `cargo metadata --format-version 1 --locked`,
not by parsing `Cargo.toml` as text, because that yields the **resolved,
inherited** value per member — without which "every member declares the same
expression" is unwritable. It is invoked in **exactly one place**: it cannot
resolve a fixture workspace in a `TempDir` (there is no `Cargo.lock`), so
every other check takes its subject as an argument — an expression string, a
README's contents, a root path — which is what lets a fixture enter the same
code path the shipped tree does instead of a re-implementation of it.

Members come from the metadata's `workspace_members` id list, not from
`packages`, which holds 355 entries of which 185 declare this same
expression. The member set is compared against the directories under
`crates/` rather than against a count, so it catches a metadata read that
resolved nothing *and* survives the workspace gaining an eleventh crate.

## What deliberately does not exist

- **No `NOTICE` file.** Apache-2.0 §4(d) obliges one only if the work already
  carries one. It does not, and inventing one commits every downstream
  redistributor to propagating it.
- **No `license-file` key.** `license` with an SPDX expression is correct;
  `license-file` is for licences with no SPDX identifier.
- **No per-crate licence files.** Members inherit
  `[workspace.package].license`.
- **No contribution-licensing prose** (the "intentionally submitted for
  inclusion" paragraph). That is a policy statement about inbound
  contributions, not a licence text.
- **No copyright headers in source files.** No source file carries one today.
- **No `deny.toml` change.** Its `[licenses] allow` list already permits
  `MIT` and `Apache-2.0`, and that stage grades *dependencies*, never this
  repository's own files.
- **No general markdown link validation.** The check reads licence-file links
  only — a target whose final path segment begins with `LICENSE`, in either
  the `[text](TARGET)` or the angle-bracket `[text](<TARGET>)` form, anywhere
  in the README rather than only inside its `## License` section.

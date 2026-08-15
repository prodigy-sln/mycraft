# Testing Standards (Constitution)

> Immutable. All generated code MUST comply. Violations require explicit
> justification and user approval.

## 1. Test-Driven Development

Every feature follows Red-Green-Refactor:

1. **RED** — a failing test derived from exactly one spec scenario. The
   scenario↔test mapping is recorded in the spec folder's `test-map.md`,
   never in test names or code. The failing output MUST be displayed
   before any implementation is written.
   Commit: `test: add failing tests for [behavior]`.
2. **GREEN** — minimal code to pass: no premature optimization, no
   "while I'm here" additions. Commit: `feat: implement [behavior]`.
3. **REFACTOR** — improve the task's own diff while tests stay green.
   Issues outside the diff are recorded as deferred observations, never
   fixed in passing. Commit only when something changed:
   `refactor: improve [component]`.

Test-first mandate: tests exist and fail before implementation begins.
Never write implementation before tests, write tests afterwards "for
coverage", or skip tests for "simple" code.

**Ownership and arbitration (rigor medium+):** a phase's tests are
authored by a test author that has not seen any implementation and owns
them for the whole phase. The implementation context never edits test
files. Disputed failures go to the test author, judged against the spec
scenario, with exactly one verdict: `test-correct` (implementation
conforms), `test-wrong` (author fixes and commits), or
`scenario-ambiguous` (user decides). At rigor `low`, tests and
implementation share one context; the displayed failing output is the
discipline gate.

Exceptions — test-first may be relaxed only for exploratory spikes (thrown
away or covered before merge), pure configuration, and generated code (the
generator is tested). Document every exception in the spec.

**What counts as RED.** A compile error is acceptable RED only when the
scenario is genuinely about a type or function existing. For a behaviour
scenario, get an *assertion* failure — implement deliberately less first
if that is what it takes to reach the assertion. A test that never ran
cannot show you it was checking the right thing.

**One skeleton is often not enough, and which one depends on the phase.**
An empty-output skeleton cannot falsify a scenario expecting zero results
— it passes for the wrong reason — so that phase needs a second,
over-eager skeleton to drive the scenario red. The inverse holds where
scenarios assert something is *not* removed: there the over-eager
skeleton is the one that passes vacuously. Pick the skeletons that make
*this phase's* scenarios fail.

## 2. Falsifiability

**Green is not evidence unless the test could have been red, for the
right reason.** Twenty-two instances of something passing for the wrong
reason are what this section is paid for, and the escalation is the point:
each was invisible to the defence that caught the one before it. Assume
the next is in the diff you are looking at.

- **Prefer a derived oracle to a committed number.** No expected quantity
  may be copied from a run of the code under test. A count snapshotted
  from the first green run commits whatever the code happened to do that
  day: an emit-nothing implementation gets `0` recorded as its expected
  count and passes forever. Derive the number by arithmetic or from an
  independent oracle that shares no code with the subject.
- **A count cannot see shape.** A fixture measuring the wrong workload
  satisfies every count-based check written against it. Fixture
  construction is a constraint no assertion can enforce, so it is held by
  the code that builds the fixture and by a reviewer reading it — say so
  where it matters rather than assuming the numbers cover it.
- **Prove a consequential pass non-vacuous by mutation.** Break the
  implementation by hand, observe the suite, revert by hand, and confirm
  `git diff --exit-code` is clean before continuing. A mutation that does
  **not** bite is evidence about the code's structure, not automatically
  a test gap — record the outcome either way, including the ones that
  did not bite.
- **A structural-invariant test needs a positive control.** A test that
  asserts only an absence (no such dependency, no such literal) goes
  green forever the day the thing it guarded against is quietly removed.
  Pair it with a separate test function asserting the same scan *does*
  report a fixture that contains the thing.
- **An over-tight assertion invites a real defect.** The failure runs the
  other way too: *red that should have been green*, whose obvious "fix" is
  to break production code. Bit equality on a camera's two declared eye
  positions would have failed against a **correct** camera —
  `f32::consts::PI` sits a hair above π, so its sine is −8.7e−8, and times
  96 that is two units in the last place of 32 — and the cheapest way to
  green it is to round the result or special-case a tick. Note the trap:
  the exact comparison was also the *consistent* one, matching every other
  test in that file. **Measure the arithmetic path before choosing the
  assertion; inspecting the literals is not enough.** Derive a tolerance
  from both directions — above the measured error, below the smallest
  difference the test must still catch — never by loosening until green.
- **Agreement between two wrong things is not evidence.** A unit test of a
  conversion, and a test comparing two configurations *to each other*, can
  both be green while every frame ships visibly wrong: neither looks at the
  value that actually reaches the device. When a decision is about what
  crosses a boundary, assert it at the boundary.
- **An absent reviewer and a clean reviewer look identical.** This applies
  to verification itself, not only to code. A verdict aggregated from
  structured output can read "no findings" because a reviewer returned
  *nothing* — the same way a summary line cannot distinguish a passing
  assertion from one that never ran. Check the per-reviewer payloads, not
  the merged result.
- **Policy is not wiring.** Testing a pure decision does not test that the
  application consults it. A client submitting a default movement intent
  every tick — no key press and no pointer motion reaching the player at
  all — left 406 of 406 green, as did deleting the guard that stops a free
  cursor turning the camera. A test that calls the same pure function the
  adapter calls is agreement between two copies of one decision; the
  adapter can stop calling it entirely and both stay green. Ask of any
  pure core: **what calls this, and what would go red if it stopped?** The
  tell is structural — a handler needing a real window, socket or device
  that nothing constructs, sitting in a layer coverage is configured not
  to count.
- **A second entry point onto a tested path is untested until something
  asserts through it.** `load_world` handed back a zeroed player and 213
  tests stayed green, because every scenario read the player through
  `saved_player` and the two share one preamble — which is exactly why the
  shared part looked covered. Separately, a launch helper ignored its
  acceptance argument entirely and 140 stayed green. Coverage says the
  code ran; it does not say anything was checking. Assert through the new
  caller, not through the one that was already covered.
- **An enumerated verdict beats an absence assertion.**
  `assert!(found.is_empty())` cannot tell an empty answer from a scan that
  can no longer look. `assert_eq!(verdict, EveryElementStatesAnOutline)`
  rejects every other verdict *including* the ones that mean "I could not
  look", so a vanished input directory reddens for free. Prefer a total
  enum verdict wherever a scan reports one. It does not retire the
  positive control above: the hole that survives is *inside* the good
  verdict, where a scan that came to return an empty list unconditionally
  would answer "all treated" forever.
- **Red for a known reason hides red for an unknown one.** A test already
  failing on a stale count also swallowed a revision-substitution defect —
  the only test in 661 that could see it. This is the same family with the
  sign flipped, and worse in one way: a known-red test invites deferral
  ("we know about that one"), which is precisely the state in which it
  stops reporting anything new. A test red for an expected reason is fixed
  before the phase closes, never annotated.
- **A green suite is no evidence about a lint.** A nesting-threshold
  defect survived 697 passing tests and two rounds of falsification: a
  suite and a lint answer different questions. The only instrument that
  can report it is the gate, and a phase opening with an adaptation commit
  has **no compilable tree for the gate to run on** until the
  implementation lands — so anything only the gate can see accumulates
  silently across that whole window. Whoever authors tests inside it runs
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  directly. Checking at a lower severity asks a different question, and
  without `-D warnings` cargo attributes the diagnostic to the first
  binary and marks the rest `(1 duplicate)` — which means *this same
  diagnostic, repeated*, not *a pre-existing one lives elsewhere*.

Test placement — sibling `foo_test.rs` files for unit tests (a considered
departure from Rust's inline default), `tests/` for integration tests,
and when each is appropriate — is recorded once in
`docs/technical/testing.md`, not repeated here.

## 3. Test Quality

- Tests are independent (run in any order), repeatable, self-contained,
  and fast (unit <100ms, integration <1s).
- Names describe behavior — `[unit]_[scenario]_[expected]` or BDD
  `it('...')` — and never contain spec or scenario IDs.
- One logical assertion per test. Assert behavior, not implementation.
  Use specific assertions with failure messages where complex.
- Tests are living documentation: names state expected behavior, setup
  shows valid inputs, assertions show expected outputs.
- Organization follows the language's own convention rather than a fixed
  directory triple; for Rust see the placement note above.

## 4. Coverage

- Minimums: business logic 90%, API endpoints 80%, UI components 70%,
  utilities 80%, overall 80%.
- 100% required: auth, payment and financial calculations, validation
  rules, security-sensitive operations.
- Exceptions only for third-party wrappers, framework boilerplate, and
  logging — configured in the coverage tool, never silently ignored.

## 5. Mocking

- Prefer real dependencies: test containers or in-memory DB, temp
  filesystem, test servers. Mock only unavailable or unreliable externals,
  specific failure scenarios, slow dependencies, and rate-limited APIs.
- Mock at boundaries and keep mocks simple — a complex mock signals a
  design problem.

## 6. Test Types

- **Unit**: no I/O, milliseconds — business logic, transformations,
  validation rules.
- **Integration**: component boundaries, database, API endpoints, auth
  flows — seconds.
- **E2E**: full system, critical user journeys only, 5–10 per feature
  maximum.

## 7. Test Data

- Minimal, obvious, set up per test. Values make intent clear
  (`invalidEmail = 'not-an-email'`) — no magic values. Use factories with
  sensible defaults for complex objects.

## 8. Continuous Integration

- Every PR: all tests pass, new functionality is tested, coverage
  thresholds met, lint clean. Never merge failing tests, disable tests to
  make CI pass, or skip a test without a tracked issue and deadline.
- Flaky tests are quarantined immediately and fixed within one sprint or
  deleted.

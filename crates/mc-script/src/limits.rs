//! Everything the host is willing to spend on script, and what it does when
//! script asks for more.
//!
//! Every limit is configurable and every one has a default, because the two
//! serve different masters: a production-sized memory cap cannot be tripped
//! inside a test's time budget, and a test-sized one would refuse ordinary
//! content on a real server. Tests configure; the defaults ship.

use std::num::{NonZeroU32, NonZeroU64, NonZeroUsize};

/// The bounds one host enforces on the script it runs.
///
/// The fields are public and the type carries a [`Default`], so raising one
/// limit for a workload that needs it is a struct update rather than a builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostLimits {
    /// How many times the backend's interrupt may fire in one entry into
    /// script before the entry is aborted.
    ///
    /// **It counts calls and loop edges, and not instructions.** The interrupt
    /// is emitted at seven opcodes — call, fastcall, return, the two `for`
    /// iterations, and the two backward jumps — so the body of a loop is free:
    /// a thousand straight-line statements cost one, and a loop of ten
    /// statements costs exactly what an empty loop costs. A call within script
    /// costs two and a call into the host costs one.
    ///
    /// The consequence for whoever sizes this, and for whoever writes content
    /// against it: **cost is reduced by batching calls, never by shortening
    /// code.**
    pub call_and_loop_budget: NonZeroU64,
    /// How many bytes of script allocation one entry may add **above the
    /// baseline it started from**.
    ///
    /// A delta rather than an absolute, because that is what makes an
    /// allocation failure attributable to the entry that caused it.
    pub memory_cap: NonZeroUsize,
    /// The absolute ceiling the whole script state may reach, enforced by the
    /// allocator rather than by the interrupt.
    ///
    /// It bounds peak allocation — a single allocation large enough to jump the
    /// gap between two interrupt ticks fails here rather than sailing past —
    /// and it is the one number that decides when scripting stops working for
    /// everybody. It must sit above the state's own baseline plus
    /// [`memory_cap`](Self::memory_cap), or the host is configured into
    /// permanent memory pressure from its first invocation.
    pub memory_backstop: NonZeroUsize,
    /// How many consecutive faulting invocations quarantine an attachment.
    ///
    /// Consecutive, and reset by a success. The budget bounds what one
    /// invocation costs; this bounds how often a broken one repeats.
    pub fault_threshold: NonZeroU32,
    /// How many invocations one dispatch round may perform before it ends and
    /// leaves the rest for the next round.
    pub round_bound: NonZeroU32,
    /// How many entries of follow-up work may be waiting at once.
    ///
    /// Separate from [`round_bound`](Self::round_bound), which bounds
    /// invocations per round and says nothing about queue length: a callback
    /// returning a fan-out grows the queue faster than a round drains it, and
    /// every entry is a host-side allocation outside every script-side limit.
    /// Work that cannot be admitted is refused and named, never silently
    /// dropped.
    pub pending_bound: NonZeroU32,
}

/// The shipped defaults, in the units their fields carry.
///
/// **This block is the one source for them.** Every other scenario in the crate
/// configures the limit it is about, because a production-sized memory cap
/// cannot be tripped inside a test's time budget and a test-sized one would
/// refuse ordinary content — so nothing else constrains what actually ships,
/// and a second place stating these numbers in prose would drift from them
/// silently. Whoever changes one changes it here, where the reason it holds is
/// written beside it.
///
/// Each is a non-zero literal, so the fallbacks in [`HostLimits::default`] are
/// unreachable. They are written rather than asserted because a field of a
/// non-zero type cannot be built from a literal without either a conversion
/// that panics — which this crate denies at its root — or an unsafe one.
///
/// The budget is sized for the largest plausible **unsliceable** workload, and
/// the reason is structural rather than an estimate: work that runs as a
/// callback has somewhere to go when it exceeds its budget — the pending queue,
/// across rounds — and chunk evaluation has nowhere, so the value is set by what
/// must fit in one entry. Against the measured cost of a 16³ pass — 4,369 ticks
/// bare, 8,465 with one host call per cell, 12,561 with one call within script
/// per cell, so roughly one, two and three ticks a cell — a chunk walking a 64³
/// volume at one host call per cell costs about 540,000. A million admits that
/// with room to spare and refuses a workload an order of magnitude past it.
const DEFAULT_CALL_AND_LOOP_BUDGET: u64 = 1_000_000;
/// A delta above the entry baseline, so its floor is what a callback plausibly
/// needs rather than what the script state weighs.
const DEFAULT_MEMORY_CAP: usize = 256 * 1024;
/// Must exceed the state's measured 385,952-byte baseline plus the cap above,
/// or the host is in permanent memory pressure from its first invocation. That
/// floor is 648,096 bytes; this sits about twenty-five times above it, which
/// leaves room for state legitimately retained across many attachments while
/// staying a number an operator can hold in their head. It is the one value
/// that decides when scripting stops working for everybody.
const DEFAULT_MEMORY_BACKSTOP: usize = 16 * 1024 * 1024;
/// Three consecutive faults, the count reset by a success. The budget bounds
/// what one invocation costs; this bounds how often a broken one repeats.
const DEFAULT_FAULT_THRESHOLD: u32 = 3;
/// Invocations one dispatch round may perform before the rest waits for the
/// next one.
const DEFAULT_ROUND_BOUND: u32 = 64;
/// Four rounds' worth of queued work at the round bound above.
const DEFAULT_PENDING_BOUND: u32 = 256;

/// The ceiling stated for `round_bound × call_and_loop_budget`, in interrupt
/// ticks.
///
/// One round may enter script [`round_bound`](HostLimits::round_bound) times and
/// each entry may spend a whole [`call_and_loop_budget`](HostLimits::call_and_loop_budget),
/// so their product is what a single round can cost. The two constraints on the
/// budget **do not covary**: it must be large enough for the biggest unsliceable
/// chunk, which tracks the largest content pack anyone ships, and small enough
/// that a round of them fits wherever rounds have to fit, which tracks something
/// else entirely. A value satisfying both today is broken by either moving.
///
/// **It is stated rather than derived, and that is the finding rather than a
/// shortcut.** Deriving the pair jointly needs a tick to derive it against, and
/// nothing calls dispatch from a tick yet. So this is written at the pair's
/// present value as an independent literal — never computed from the two
/// defaults, which would make it follow them wherever they went and bound
/// nothing. Raising either default past it is then a deliberate act, and the
/// spec that first dispatches from a tick owns re-deriving all three quantities
/// together: chunk budget, callback budget, round bound.
pub const PROVISIONAL_ROUND_BUDGET_CEILING: u64 = 64_000_000;

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            call_and_loop_budget: NonZeroU64::new(DEFAULT_CALL_AND_LOOP_BUDGET)
                .unwrap_or(NonZeroU64::MIN),
            memory_cap: NonZeroUsize::new(DEFAULT_MEMORY_CAP).unwrap_or(NonZeroUsize::MIN),
            memory_backstop: NonZeroUsize::new(DEFAULT_MEMORY_BACKSTOP)
                .unwrap_or(NonZeroUsize::MIN),
            fault_threshold: NonZeroU32::new(DEFAULT_FAULT_THRESHOLD).unwrap_or(NonZeroU32::MIN),
            round_bound: NonZeroU32::new(DEFAULT_ROUND_BOUND).unwrap_or(NonZeroU32::MIN),
            pending_bound: NonZeroU32::new(DEFAULT_PENDING_BOUND).unwrap_or(NonZeroU32::MIN),
        }
    }
}

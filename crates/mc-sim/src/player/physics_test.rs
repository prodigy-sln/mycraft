//! The tick quantum is one number, and it is spelled twice.
//!
//! Everything else in this module is graded by the suites that drive a whole
//! simulation. What no scenario can see is the two constants drifting apart: the
//! physics multiplies a velocity by [`TICK_DURATION`] seconds and a frame path
//! subtracts [`TICK_QUANTUM`] from an elapsed interval, and a client that spent
//! quanta of one length into a world that simulated the other would run slow or
//! fast by exactly the ratio — the defect the pacing exists to remove, in
//! miniature and with every behavioural test still green.
//!
//! # The tolerance, from both directions
//!
//! The floor is representation error, and it is not zero. A sixtieth of a second
//! is 16 666 666.67 ns; the `Duration` is written to the nearest nanosecond and
//! the `f32` cannot hold the value at all — one unit in its last place at this
//! magnitude is 0.93 ns, and `Duration::from_secs_f32` then rounds that to a whole
//! nanosecond of its own. Measured rather than assumed: the two land 2 ns apart,
//! so an exact comparison reds a correct pair and did.
//!
//! The ceiling is what a disagreement would cost. 2 ns a tick is 0.43 ms over an
//! hour of play, which no player and no golden frame can see. The next error that
//! is not representation — a digit in any place but the last, a wrong divisor, a
//! millisecond written where a nanosecond was meant — moves the pair by 10 ns at
//! the very least, and by six orders of magnitude at the likeliest. 2 ns sits
//! above the floor and five times below the ceiling.

use std::time::Duration;

use super::{TICK_DURATION, TICK_QUANTUM};

/// How far the two spellings may sit apart. See this file's header.
const WITHIN: Duration = Duration::from_nanos(2);

#[test]
fn the_quantum_a_frame_path_spends_is_the_same_length_as_the_tick_it_buys() {
    let declared = Duration::from_secs_f32(TICK_DURATION);

    let apart = declared
        .checked_sub(TICK_QUANTUM)
        .unwrap_or_else(|| TICK_QUANTUM.saturating_sub(declared));

    assert!(
        apart <= WITHIN,
        "a frame path spends {TICK_QUANTUM:?} of elapsed time to buy one tick, and the tick it \
         buys simulates {declared:?}. They are the same number written for two different \
         consumers, so a client whose quanta are longer than its ticks runs the world slow and \
         one whose quanta are shorter runs it fast — by exactly the ratio, silently, with every \
         scenario about where a walk ends still green. They sit {apart:?} apart"
    );
}

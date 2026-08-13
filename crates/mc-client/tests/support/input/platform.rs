//! A platform that answers a request for the pointer the way a declared list
//! says, and writes down everything it was asked.
//!
//! **It records every ask, including the ones nothing currently reads.** A
//! double that logged only what somebody asserts today lies by omission the day
//! somebody asserts something else — and the release and the cursor-visibility
//! asks are exactly the ones a client could stop making without any scenario
//! noticing.
//!
//! Granting is decided by membership of a declared list rather than by a flag,
//! so a platform that refuses the first capture and grants the second is one
//! constructor argument away. A double that granted whatever it was asked would
//! make the ladder's second rung unreachable while every scenario stayed green.

use std::cell::RefCell;
use std::rc::Rc;

use mc_client::session::{PointerAsk, PointerPlatform};
use mc_render::window::CaptureState;

/// Everything a platform was asked for, in the order it was asked.
///
/// Shared with the harness rather than handed back at the end, because the
/// session owns its platform opaquely once it is built. Single-threaded, as the
/// session and the client's own event loop both are.
#[derive(Debug, Clone, Default)]
pub struct PointerLog(Rc<RefCell<Vec<PointerAsk>>>);

impl PointerLog {
    /// Writes one ask down.
    fn record(&self, ask: PointerAsk) {
        self.0.borrow_mut().push(ask);
    }

    /// Everything asked so far.
    pub fn asks(&self) -> Vec<PointerAsk> {
        self.0.borrow().clone()
    }
}

/// The pointer as a declared platform holds it.
#[derive(Debug)]
pub struct RecordingPlatform {
    granted: Vec<CaptureState>,
    log: PointerLog,
}

impl RecordingPlatform {
    /// A platform that grants exactly the captures in `granted` and refuses
    /// every other, and the log it will write to.
    pub fn granting(granted: &[CaptureState]) -> (Self, PointerLog) {
        let log = PointerLog::default();
        let platform = Self {
            granted: granted.to_vec(),
            log: log.clone(),
        };
        (platform, log)
    }
}

impl PointerPlatform for RecordingPlatform {
    fn grab(&mut self, capture: CaptureState) -> bool {
        self.log.record(PointerAsk::Grab(capture));
        self.granted.contains(&capture)
    }

    fn release(&mut self) {
        self.log.record(PointerAsk::Release);
    }

    fn show_cursor(&mut self, visible: bool) {
        self.log.record(PointerAsk::CursorVisible(visible));
    }
}

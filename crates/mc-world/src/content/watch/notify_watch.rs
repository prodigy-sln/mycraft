//! The one file in this workspace that names a filesystem-watching vendor.
//!
//! **Litmus test: if `notify` disappeared tomorrow, one file changes.** Nothing
//! above the port sees a vendor type, and every decision about what a change
//! *means* lives on the other side of it — this is construction, drain and map,
//! and deliberately nothing else. That is also what keeps it honest in a crate
//! that is in the coverage denominator: there is very little here to cover.
//!
//! **The clock is here and only here.** The debouncer reads one; the simulation
//! reads none.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use super::{ContentChanges, ContentWatch, SETTLING_WINDOW};

/// A content root watched through `notify`'s debouncer.
#[derive(Debug)]
pub struct NotifyContentWatch {
    /// The live debouncer and the channel it reports through, together because
    /// **the debouncer is held for its `Drop`**: releasing it stops the watch, so
    /// the receiver outliving it would drain a channel nothing feeds.
    ///
    /// `None` where the root could not be watched at all, which is reported once
    /// and then yields no changes — nothing in the spec asks for a retry.
    watching: Option<(
        Debouncer<notify::RecommendedWatcher, RecommendedCache>,
        Receiver<DebounceEventResult>,
    )>,
    window: Duration,
    /// The refusal this root could not be watched with, taken by the first ask.
    unwatchable: Option<(PathBuf, String)>,
}

impl NotifyContentWatch {
    /// Watches `root`, letting a save settle for the declared window.
    ///
    /// **The one place the declared window is supplied.**
    #[must_use]
    pub fn watching(root: &Path) -> Self {
        Self::settling_for(root, SETTLING_WINDOW)
    }

    /// The same, settling for `window` instead.
    ///
    /// Exists so the window can be asserted where it crosses into the vendor,
    /// which offers no accessor for its own timeout. A test comparing two
    /// different windows is what makes [`settling_window`](Self::settling_window)
    /// report what was handed over rather than a constant.
    #[must_use]
    pub fn settling_for(root: &Path, window: Duration) -> Self {
        let (sender, events) = channel();
        match new_debouncer(window, None, move |reported| {
            // A closed receiver means the watch was dropped; there is nobody left
            // to tell and nothing to recover.
            drop(sender.send(reported));
        }) {
            Ok(mut debouncer) => match debouncer.watch(root, RecursiveMode::Recursive) {
                Ok(()) => Self::watching_with(debouncer, events, window),
                Err(refused) => Self::unwatchable(root, window, &refused),
            },
            Err(refused) => Self::unwatchable(root, window, &refused),
        }
    }

    /// The window this watch handed its debouncer.
    #[must_use]
    pub const fn settling_window(&self) -> Duration {
        self.window
    }

    /// A watch holding a live debouncer.
    fn watching_with(
        debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
        events: Receiver<DebounceEventResult>,
        window: Duration,
    ) -> Self {
        Self {
            watching: Some((debouncer, events)),
            window,
            unwatchable: None,
        }
    }

    /// A watch over a root that could not be watched.
    fn unwatchable(root: &Path, window: Duration, cause: &notify::Error) -> Self {
        Self {
            watching: None,
            window,
            unwatchable: Some((root.to_owned(), cause.to_string())),
        }
    }
}

impl ContentWatch for NotifyContentWatch {
    /// Every path the debouncer has reported since this was last asked.
    ///
    /// An unwatchable root answers with its refusal **once** and with
    /// [`ContentChanges::Nothing`] thereafter, so a caller reports it once
    /// without holding a flag of its own.
    fn changes(&mut self) -> ContentChanges {
        if let Some((directory, cause)) = self.unwatchable.take() {
            return ContentChanges::Unwatchable { directory, cause };
        }
        let mut changed = Vec::new();
        while let Some(batch) = self.next_batch() {
            changed.extend(batch);
        }
        if changed.is_empty() {
            ContentChanges::Nothing
        } else {
            ContentChanges::Changed(changed)
        }
    }
}

impl NotifyContentWatch {
    /// The next batch of paths the debouncer has ready, or nothing where the
    /// channel is empty or gone.
    ///
    /// A batch the vendor could not produce is passed over rather than reported:
    /// the loader reads the whole root on any change, so a lost event is at worst
    /// a reload that waits for the next save, and there is nothing here a caller
    /// could act on.
    fn next_batch(&mut self) -> Option<Vec<PathBuf>> {
        let (_debouncer, events) = self.watching.as_ref()?;
        match events.try_recv() {
            Ok(Ok(events)) => Some(
                events
                    .into_iter()
                    .flat_map(|event| event.event.paths.clone())
                    .collect(),
            ),
            Ok(Err(_)) => Some(Vec::new()),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

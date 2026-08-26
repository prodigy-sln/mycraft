//! Elapsed time, as the thing that draws frames measures it.
//!
//! One port and one adapter, and the reason they are a crate away from the
//! client that reads them is the wall-clock confinement scan: the client and the
//! renderer are both forbidden to name a system clock, save the single file
//! below whose whole job is to be one.
//!
//! Nothing here is about drawing. What it answers — how long since the last
//! frame — is what the client's frame path spends into simulation ticks, and the
//! debug overlay's frame rate is the same reading shown the other way round.

pub mod clock;

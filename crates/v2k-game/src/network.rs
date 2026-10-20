//! V2000's network play ([MULTIPLAYER.md](../../../docs/re/MULTIPLAYER.md)).
//!
//! Retail plays it over DirectPlay 3; the port's [`transport`] takes
//! DirectPlay's place. Everything above it follows retail: the
//! [`framing`] of records into messages, the [`slots`] and [`handles`]
//! that name players and bodies, and the game protocol.

pub mod arena;
pub mod framing;
pub mod game;
pub mod handles;
pub mod protocol;
pub mod scoring;
pub mod slots;
pub mod transport;

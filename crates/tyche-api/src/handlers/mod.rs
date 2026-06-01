//! HTTP handlers.
//!
//! Handlers are intentionally thin: parse → call the core crate → record a
//! metric → return JSON. All business logic lives in `tyche-sim` /
//! `tyche-attest`; the API layer only adapts it to HTTP.

pub mod attest;
pub mod health;
pub mod simulate;
pub mod verify;

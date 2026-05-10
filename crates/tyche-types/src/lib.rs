//! L1 data graph for Tyche.
//!
//! This crate defines the canonical wire types for portfolios, loans, covenants,
//! macro scenarios, and risk metrics, along with helpers for deterministic
//! serialization and hashing.
//!
//! Every other crate in the workspace depends on these types. They are the
//! schema that the simulation core consumes, the attestation layer commits to,
//! and the web frontend renders.
//!
//! # Stability
//!
//! Types in this crate participate in cryptographic commitments. A breaking
//! change to a field name, type, or default value alters the canonical hash of
//! every prior input and must be paired with a `model_version` bump on the
//! attestation side.

#![forbid(unsafe_code)]

pub mod canonical;
pub mod hashing;
pub mod loan;
pub mod metrics;
pub mod portfolio;
pub mod scenario;

pub use crate::canonical::canonical_json;
pub use crate::hashing::{hash_object, sha256_hex};
pub use crate::loan::{Covenant, CovenantDirection, Geography, Loan, Sector, Seniority};
pub use crate::metrics::{RiskMetrics, SectorContribution};
pub use crate::portfolio::Portfolio;
pub use crate::scenario::{MacroScenario, SectorShock};

/// Errors raised by `tyche-types`.
#[derive(Debug, thiserror::Error)]
pub enum TypesError {
    /// The provided JSON could not be parsed against the expected schema.
    #[error("invalid json: {0}")]
    Json(#[from] serde_json::Error),

    /// A semantic invariant on a value was violated (e.g. negative principal).
    #[error("invariant violated: {0}")]
    Invariant(String),
}

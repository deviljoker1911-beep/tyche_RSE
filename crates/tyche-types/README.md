# tyche-types

Layer 1 of the Tyche stack: the canonical data graph.

Defines the wire types for loans, portfolios, scenarios, and risk metrics, and the
canonical-JSON / hashing helpers used by every other crate to derive deterministic
content hashes.

These types are deliberately allocation-friendly and `Serialize + Deserialize`. They
are the single source of truth for the structure of a Tyche simulation input.

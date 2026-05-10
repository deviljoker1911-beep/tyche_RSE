//! Risk metrics output by the simulation core.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::loan::Sector;

/// Sector-level decomposition of expected loss.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectorContribution {
    /// Expected loss attributable to this sector.
    pub expected_loss: f64,
    /// Share of total portfolio expected loss (0..1).
    pub share: f64,
}

/// Output of a single simulation run.
///
/// All loss figures are in the portfolio's reporting currency. `var_95`,
/// `var_99` and `expected_shortfall_975` are expressed as positive losses
/// (i.e. a `var_99` of 12.4 means a 99th-percentile loss of 12.4 units), per
/// the Basel convention.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskMetrics {
    /// Mean loss across all simulated paths.
    pub expected_loss: f64,
    /// Value at Risk at the 95% level (positive number = larger loss).
    pub var_95: f64,
    /// Value at Risk at the 99% level.
    pub var_99: f64,
    /// Expected Shortfall (CVaR) at the 97.5% level.
    pub expected_shortfall_975: f64,
    /// Per-sector decomposition. Keys sum to total `expected_loss` in `expected_loss`,
    /// and `share` fields sum to ~1.0 modulo float error.
    pub sector_contributions: BTreeMap<Sector, SectorContribution>,
    /// Number of simulated paths used to estimate these metrics.
    pub n_paths: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut sectors = BTreeMap::new();
        sectors.insert(
            Sector::Healthcare,
            SectorContribution {
                expected_loss: 1.2,
                share: 0.4,
            },
        );
        let m = RiskMetrics {
            expected_loss: 3.0,
            var_95: 5.0,
            var_99: 8.0,
            expected_shortfall_975: 9.5,
            sector_contributions: sectors,
            n_paths: 20_000,
        };
        let s = serde_json::to_string(&m).unwrap();
        let m2: RiskMetrics = serde_json::from_str(&s).unwrap();
        assert_eq!(m, m2);
    }
}

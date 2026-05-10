//! Macro scenario types.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::loan::Sector;

/// A directional shock applied to a single sector.
///
/// Components are expressed as fractional changes: 0.10 = +10%. Negative values
/// indicate adverse shocks (e.g. demand contraction, EBITDA erosion).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SectorShock {
    /// Shock to expected EBITDA for issuers in the sector.
    pub ebitda_shock: f64,
    /// Shock to liquid asset value (mark-to-market write-down on collateral).
    pub asset_shock: f64,
}

impl SectorShock {
    /// A neutral shock (no perturbation).
    #[must_use]
    pub const fn neutral() -> Self {
        Self {
            ebitda_shock: 0.0,
            asset_shock: 0.0,
        }
    }
}

/// A named macro scenario.
///
/// `gdp_shock` is fractional (-0.05 = -5% GDP), `rate_shock_bps` is in basis
/// points applied to the front-end of the curve. Sector-level shocks are
/// keyed by `Sector` and override the macro-level defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroScenario {
    /// Stable identifier; used in attestation records.
    pub scenario_id: String,
    /// Human-readable name.
    pub name: String,
    /// Aggregate GDP shock (fractional).
    pub gdp_shock: f64,
    /// Parallel front-end rate shock in basis points.
    pub rate_shock_bps: f64,
    /// Sector-level overrides. Missing sectors fall back to the macro defaults
    /// derived from `gdp_shock`.
    #[serde(default)]
    pub sector_shocks: BTreeMap<Sector, SectorShock>,
}

impl MacroScenario {
    /// Resolve the effective shock for a sector under this scenario.
    ///
    /// If `sector_shocks` contains an explicit entry, that wins. Otherwise the
    /// scenario's macro `gdp_shock` is broadcast to both EBITDA and asset
    /// channels with a 1.0x and 0.6x multiplier respectively (asset values
    /// are typically more sticky than earnings in a downturn).
    #[must_use]
    pub fn effective_shock(&self, sector: Sector) -> SectorShock {
        if let Some(s) = self.sector_shocks.get(&sector) {
            return *s;
        }
        SectorShock {
            ebitda_shock: self.gdp_shock,
            asset_shock: self.gdp_shock * 0.6,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut sectors = BTreeMap::new();
        sectors.insert(
            Sector::Energy,
            SectorShock {
                ebitda_shock: -0.20,
                asset_shock: -0.10,
            },
        );
        let s = MacroScenario {
            scenario_id: "stagflation_severe".into(),
            name: "Stagflation severe".into(),
            gdp_shock: -0.04,
            rate_shock_bps: 250.0,
            sector_shocks: sectors,
        };
        let j = serde_json::to_string(&s).unwrap();
        let s2: MacroScenario = serde_json::from_str(&j).unwrap();
        assert_eq!(s, s2);
    }

    #[test]
    fn explicit_sector_override_wins() {
        let mut sectors = BTreeMap::new();
        sectors.insert(
            Sector::Energy,
            SectorShock {
                ebitda_shock: -0.5,
                asset_shock: -0.3,
            },
        );
        let s = MacroScenario {
            scenario_id: "x".into(),
            name: "x".into(),
            gdp_shock: -0.02,
            rate_shock_bps: 0.0,
            sector_shocks: sectors,
        };
        assert!((s.effective_shock(Sector::Energy).ebitda_shock + 0.5).abs() < 1e-12);
        // Falls back to gdp_shock for unmapped sectors.
        assert!((s.effective_shock(Sector::Telecom).ebitda_shock + 0.02).abs() < 1e-12);
    }
}

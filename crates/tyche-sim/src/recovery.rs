//! Recovery models.

use serde::{Deserialize, Serialize};
use tyche_types::{Loan, SectorShock};

/// Recovery model used by the simulator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryModel {
    /// Constant recovery equal to the seniority floor, ignoring collateral
    /// and macro state. Useful as a sanity baseline.
    SeniorityFloor,
    /// Seniority floor uplifted by collateral coverage and dampened by the
    /// scenario's asset-shock channel. The default model used in production.
    SeniorityCollateralized,
}

impl RecoveryModel {
    /// Compute the recovery rate for a defaulted loan under a given shock.
    #[must_use]
    pub fn rate(self, loan: &Loan, shock: SectorShock) -> f64 {
        let base = loan.seniority.base_recovery_rate();
        match self {
            Self::SeniorityFloor => base.clamp(0.0, 0.95),
            Self::SeniorityCollateralized => {
                // Collateral above 1.0 lifts recovery linearly toward 0.95;
                // below 1.0 it pulls recovery toward zero. The asset shock
                // erodes whichever portion is collateral-driven.
                let cc = loan.collateral_coverage.clamp(0.0, 2.0);
                let collateral_uplift = ((cc - 1.0) * 0.30).clamp(-0.40, 0.30);
                let raw = base + collateral_uplift;
                let dampen = (1.0 + shock.asset_shock).clamp(0.5, 1.0);
                (raw * dampen).clamp(0.0, 0.95)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tyche_types::{Covenant, CovenantDirection, Geography, Sector, Seniority};

    fn loan(senior: Seniority, cc: f64) -> Loan {
        Loan {
            loan_id: "L".into(),
            issuer: "I".into(),
            sector: Sector::Healthcare,
            geography: Geography::EuCore,
            seniority: senior,
            principal: 1.0,
            coupon: 0.07,
            maturity_years: 5.0,
            leverage: 4.0,
            asset_volatility: 0.30,
            covenants: vec![Covenant {
                name: "x".into(),
                threshold: 5.0,
                direction: CovenantDirection::LeqThreshold,
                cushion: 0.20,
            }],
            collateral_coverage: cc,
        }
    }

    #[test]
    fn collateralised_above_floor() {
        let l = loan(Seniority::SeniorSecured, 1.5);
        let r = RecoveryModel::SeniorityCollateralized.rate(&l, SectorShock::neutral());
        assert!(r > Seniority::SeniorSecured.base_recovery_rate());
    }

    #[test]
    fn under_collateralised_below_floor() {
        let l = loan(Seniority::SeniorSecured, 0.5);
        let r = RecoveryModel::SeniorityCollateralized.rate(&l, SectorShock::neutral());
        assert!(r < Seniority::SeniorSecured.base_recovery_rate());
    }

    #[test]
    fn asset_shock_dampens_recovery() {
        let l = loan(Seniority::SeniorSecured, 1.5);
        let no_shock = RecoveryModel::SeniorityCollateralized.rate(&l, SectorShock::neutral());
        let bad_shock = RecoveryModel::SeniorityCollateralized.rate(
            &l,
            SectorShock {
                ebitda_shock: -0.2,
                asset_shock: -0.30,
            },
        );
        assert!(bad_shock < no_shock);
    }
}

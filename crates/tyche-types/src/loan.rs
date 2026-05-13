//! Loan-level domain types.

use serde::{Deserialize, Serialize};

/// Capital-stack seniority of a credit instrument.
///
/// Ordering is from senior (lowest variant) to most subordinated. The numeric
/// ordering is exploited by the recovery model in `tyche-sim`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Seniority {
    /// Senior secured first lien.
    SeniorSecured,
    /// Senior secured second lien.
    SecondLien,
    /// Senior unsecured.
    SeniorUnsecured,
    /// Subordinated.
    Subordinated,
    /// Mezzanine.
    Mezzanine,
    /// Equity-like / payment-in-kind.
    Equity,
}

impl Seniority {
    /// Base recovery rate floor for a fully unsecured exposure of this seniority.
    ///
    /// These priors are calibrated against published European mid-market direct
    /// lending recovery studies. They are intentionally conservative.
    #[must_use]
    pub const fn base_recovery_rate(self) -> f64 {
        match self {
            Self::SeniorSecured => 0.65,
            Self::SecondLien => 0.45,
            Self::SeniorUnsecured => 0.40,
            Self::Subordinated => 0.25,
            Self::Mezzanine => 0.15,
            Self::Equity => 0.05,
        }
    }
}

/// Real-economy sector classification.
///
/// Coarse-grained on purpose: the simulation uses sector as a correlation
/// bucket and as the dimension for macro shocks. Finer NAICS / NACE codes
/// belong on the issuer record, not on the loan.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sector {
    /// Software, IT services, and digital infrastructure.
    Technology,
    /// Healthcare providers, services, and medtech.
    Healthcare,
    /// Industrial manufacturing and engineering.
    Industrials,
    /// Consumer-facing goods and services.
    Consumer,
    /// Financial services excluding banks (credit funds, insurers, etc.).
    Financials,
    /// Energy production, transmission, and oilfield services.
    Energy,
    /// Real estate, REITs, and infrastructure.
    RealEstate,
    /// Materials, chemicals, mining.
    Materials,
    /// Telecommunications and media.
    Telecom,
    /// Utilities (regulated).
    Utilities,
    /// Anything not covered above.
    Other,
}

/// Coarse geographic classification used for shock application.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Geography {
    /// United Kingdom.
    Uk,
    /// Eurozone core (DE, FR, NL, BE, AT, IE, FI, LU).
    EuCore,
    /// Eurozone periphery (IT, ES, PT, GR, plus the Baltics).
    EuPeriphery,
    /// Nordics outside the eurozone (SE, NO, DK).
    Nordics,
    /// Switzerland.
    Switzerland,
    /// Eastern Europe outside the eurozone.
    EmEurope,
    /// Anywhere else.
    Other,
}

/// Direction of a covenant test.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CovenantDirection {
    /// The metric must stay **below or equal to** `threshold`.
    /// Example: net leverage <= 5.5x.
    LeqThreshold,
    /// The metric must stay **above or equal to** `threshold`.
    /// Example: interest coverage >= 2.0x.
    GeqThreshold,
}

/// A maintenance covenant, observed periodically.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Covenant {
    /// Human-readable name (e.g. "net_leverage", "interest_coverage").
    pub name: String,
    /// Threshold value the metric is tested against.
    pub threshold: f64,
    /// Direction of the test.
    pub direction: CovenantDirection,
    /// Headroom expressed as a fraction (0.10 = 10% cushion to threshold).
    /// Negative cushion means the borrower is already in breach.
    pub cushion: f64,
}

/// A single private-credit loan.
///
/// Field semantics:
///
/// - `principal` and `coupon` are in the issuer's reporting currency. The spike
///   does not model FX explicitly; downstream code must aggregate within a
///   single currency or apply FX externally.
/// - `leverage` is net leverage at as-of date (debt / EBITDA).
/// - `asset_volatility` is the annualised volatility of the underlying firm
///   asset value, used to derive default probabilities under a Merton-style
///   structural model. Typical values: 0.20–0.45.
/// - `collateral_coverage` is the ratio of pledged collateral value to
///   `principal`. Values >1.0 indicate over-collateralisation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loan {
    /// Stable opaque identifier. Choose something the firm can re-derive
    /// (e.g. a deterministic hash of internal IDs); never raw PII.
    pub loan_id: String,
    /// Stable opaque identifier for the borrowing entity.
    pub issuer: String,
    /// Sector classification.
    pub sector: Sector,
    /// Geography classification.
    pub geography: Geography,
    /// Capital-stack seniority.
    pub seniority: Seniority,
    /// Outstanding principal at as-of date.
    pub principal: f64,
    /// Coupon as an annual fraction (0.085 = 850 bps).
    pub coupon: f64,
    /// Years to maturity from as-of date. Fractional values are allowed.
    pub maturity_years: f64,
    /// Net leverage (debt / EBITDA) at as-of date.
    pub leverage: f64,
    /// Annualised asset volatility used in the structural default model.
    pub asset_volatility: f64,
    /// Maintenance covenants attached to this loan.
    #[serde(default)]
    pub covenants: Vec<Covenant>,
    /// Collateral coverage ratio (collateral value / principal).
    pub collateral_coverage: f64,
}

impl Loan {
    /// Validate semantic invariants. Use after deserialisation.
    pub fn validate(&self) -> Result<(), crate::TypesError> {
        if self.principal <= 0.0 {
            return Err(crate::TypesError::Invariant(format!(
                "loan {}: principal must be positive, got {}",
                self.loan_id, self.principal
            )));
        }
        if !(0.0..=1.0).contains(&self.coupon) {
            return Err(crate::TypesError::Invariant(format!(
                "loan {}: coupon must lie in [0, 1], got {}",
                self.loan_id, self.coupon
            )));
        }
        if self.maturity_years <= 0.0 {
            return Err(crate::TypesError::Invariant(format!(
                "loan {}: maturity_years must be positive, got {}",
                self.loan_id, self.maturity_years
            )));
        }
        if self.asset_volatility <= 0.0 {
            return Err(crate::TypesError::Invariant(format!(
                "loan {}: asset_volatility must be positive, got {}",
                self.loan_id, self.asset_volatility
            )));
        }
        if self.collateral_coverage < 0.0 {
            return Err(crate::TypesError::Invariant(format!(
                "loan {}: collateral_coverage must be non-negative, got {}",
                self.loan_id, self.collateral_coverage
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Loan {
        Loan {
            loan_id: "L-1".into(),
            issuer: "I-1".into(),
            sector: Sector::Healthcare,
            geography: Geography::EuCore,
            seniority: Seniority::SeniorSecured,
            principal: 10_000_000.0,
            coupon: 0.075,
            maturity_years: 5.0,
            leverage: 4.5,
            asset_volatility: 0.30,
            covenants: vec![Covenant {
                name: "net_leverage".into(),
                threshold: 5.5,
                direction: CovenantDirection::LeqThreshold,
                cushion: 0.18,
            }],
            collateral_coverage: 1.4,
        }
    }

    #[test]
    fn seniority_ordering() {
        assert!(Seniority::SeniorSecured < Seniority::Equity);
        assert!(
            Seniority::SeniorSecured.base_recovery_rate() > Seniority::Equity.base_recovery_rate()
        );
    }

    #[test]
    fn validate_accepts_well_formed_loan() {
        sample().validate().unwrap();
    }

    #[test]
    fn validate_rejects_negative_principal() {
        let mut bad = sample();
        bad.principal = -1.0;
        assert!(bad.validate().is_err());
    }

    #[test]
    fn validate_rejects_out_of_band_coupon() {
        let mut bad = sample();
        bad.coupon = 1.5;
        assert!(bad.validate().is_err());
    }

    #[test]
    fn round_trip_serialization() {
        let l = sample();
        let s = serde_json::to_string(&l).unwrap();
        let l2: Loan = serde_json::from_str(&s).unwrap();
        assert_eq!(l, l2);
    }
}

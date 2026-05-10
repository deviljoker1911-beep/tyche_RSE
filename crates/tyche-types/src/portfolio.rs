//! Portfolio aggregate type.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::TypesError;
use crate::loan::{Geography, Loan, Sector};

/// A snapshot of a firm's private-credit portfolio at a point in time.
///
/// The `as_of` field is an ISO-8601 date string (`YYYY-MM-DD`). It is captured
/// as a string rather than a typed date so that serialised portfolios remain
/// hashable across language boundaries without requiring a shared chrono epoch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portfolio {
    /// Opaque, stable firm identifier. The federation layer commits to a
    /// `firm_id_hash`; the cleartext `firm_id` here is used only by the firm
    /// itself.
    pub firm_id: String,
    /// As-of date in ISO-8601 (`YYYY-MM-DD`) form.
    pub as_of: String,
    /// Loans in the portfolio.
    pub loans: Vec<Loan>,
}

impl Portfolio {
    /// Validate every loan and basic portfolio-level invariants.
    pub fn validate(&self) -> Result<(), TypesError> {
        if self.loans.is_empty() {
            return Err(TypesError::Invariant(format!(
                "portfolio {} must contain at least one loan",
                self.firm_id
            )));
        }
        for loan in &self.loans {
            loan.validate()?;
        }
        Ok(())
    }

    /// Total outstanding principal across the portfolio.
    #[must_use]
    pub fn total_exposure(&self) -> f64 {
        self.loans.iter().map(|l| l.principal).sum()
    }

    /// Number of unique issuers in the book.
    #[must_use]
    pub fn issuer_count(&self) -> usize {
        let mut issuers: Vec<&str> = self.loans.iter().map(|l| l.issuer.as_str()).collect();
        issuers.sort_unstable();
        issuers.dedup();
        issuers.len()
    }

    /// Aggregate exposure by sector. Output keys are stable and sorted.
    #[must_use]
    pub fn exposure_by_sector(&self) -> BTreeMap<Sector, f64> {
        let mut out = BTreeMap::new();
        for loan in &self.loans {
            *out.entry(loan.sector).or_insert(0.0) += loan.principal;
        }
        out
    }

    /// Aggregate exposure by geography.
    #[must_use]
    pub fn exposure_by_geography(&self) -> BTreeMap<Geography, f64> {
        let mut out = BTreeMap::new();
        for loan in &self.loans {
            *out.entry(loan.geography).or_insert(0.0) += loan.principal;
        }
        out
    }

    /// Top-`k` issuers by total exposure, descending.
    #[must_use]
    pub fn top_issuers(&self, k: usize) -> Vec<(String, f64)> {
        let mut by_issuer: BTreeMap<String, f64> = BTreeMap::new();
        for loan in &self.loans {
            *by_issuer.entry(loan.issuer.clone()).or_insert(0.0) += loan.principal;
        }
        let mut v: Vec<(String, f64)> = by_issuer.into_iter().collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(k);
        v
    }
}

// Sector and Geography need to be Ord for use as BTreeMap keys. Implement via
// a stable string encoding so the order is portable rather than dependent on
// enum declaration order.
macro_rules! impl_ord_via_serde {
    ($t:ty) => {
        impl PartialOrd for $t {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }
        impl Ord for $t {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                let a = serde_json::to_string(self).unwrap_or_default();
                let b = serde_json::to_string(other).unwrap_or_default();
                a.cmp(&b)
            }
        }
    };
}

impl_ord_via_serde!(Sector);
impl_ord_via_serde!(Geography);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loan::{Covenant, CovenantDirection, Seniority};

    fn loan(id: &str, issuer: &str, principal: f64, sector: Sector, geo: Geography) -> Loan {
        Loan {
            loan_id: id.into(),
            issuer: issuer.into(),
            sector,
            geography: geo,
            seniority: Seniority::SeniorSecured,
            principal,
            coupon: 0.07,
            maturity_years: 5.0,
            leverage: 4.0,
            asset_volatility: 0.3,
            covenants: vec![Covenant {
                name: "net_leverage".into(),
                threshold: 5.5,
                direction: CovenantDirection::LeqThreshold,
                cushion: 0.2,
            }],
            collateral_coverage: 1.3,
        }
    }

    fn portfolio() -> Portfolio {
        Portfolio {
            firm_id: "F-1".into(),
            as_of: "2026-03-31".into(),
            loans: vec![
                loan("L-1", "I-1", 10.0, Sector::Healthcare, Geography::EuCore),
                loan("L-2", "I-2", 20.0, Sector::Technology, Geography::Uk),
                loan("L-3", "I-1", 15.0, Sector::Healthcare, Geography::EuCore),
            ],
        }
    }

    #[test]
    fn aggregate_helpers() {
        let p = portfolio();
        assert!((p.total_exposure() - 45.0).abs() < 1e-9);
        assert_eq!(p.issuer_count(), 2);
        let sec = p.exposure_by_sector();
        assert!((sec[&Sector::Healthcare] - 25.0).abs() < 1e-9);
        assert!((sec[&Sector::Technology] - 20.0).abs() < 1e-9);
        let top = p.top_issuers(2);
        assert_eq!(top[0].0, "I-1");
        assert!((top[0].1 - 25.0).abs() < 1e-9);
    }

    #[test]
    fn round_trip() {
        let p = portfolio();
        let s = serde_json::to_string(&p).unwrap();
        let p2: Portfolio = serde_json::from_str(&s).unwrap();
        assert_eq!(p, p2);
    }

    #[test]
    fn validate_rejects_empty_book() {
        let p = Portfolio {
            firm_id: "F-1".into(),
            as_of: "2026-03-31".into(),
            loans: vec![],
        };
        assert!(p.validate().is_err());
    }
}

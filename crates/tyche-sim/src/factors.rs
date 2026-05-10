//! Three-factor decomposition for correlated default draws.
//!
//! Each loan's standardised firm-value innovation is modeled as
//!
//! ```text
//! z = sqrt(rho_m) · z_market + sqrt(rho_s) · z_sector + sqrt(1 - rho_m - rho_s) · z_idio
//! ```
//!
//! with `rho_m, rho_s in [0, 1]` and `rho_m + rho_s <= 1`. This is the same
//! Gaussian copula construction used in Basel-style portfolio models, just
//! with a market and sector layer rather than a single systemic factor.

/// Three-factor combiner.
#[derive(Debug, Clone, Copy)]
pub struct FactorModel {
    sqrt_market: f64,
    sqrt_sector: f64,
    sqrt_idio: f64,
}

impl FactorModel {
    /// Construct a model with the given correlation parameters.
    ///
    /// If `market + sector > 1`, both are scaled down proportionally so the
    /// idiosyncratic weight is non-negative. This keeps misconfigured callers
    /// from producing nonsensical negative variance contributions.
    #[must_use]
    pub fn new(market_corr: f64, sector_corr: f64) -> Self {
        let total = market_corr + sector_corr;
        let (m, s) = if total > 1.0 {
            (market_corr / total, sector_corr / total)
        } else {
            (market_corr, sector_corr)
        };
        let idio = 1.0 - m - s;
        Self {
            sqrt_market: m.sqrt(),
            sqrt_sector: s.sqrt(),
            sqrt_idio: idio.max(0.0).sqrt(),
        }
    }

    /// Combine three independent standard-normal draws.
    #[must_use]
    pub fn combine(self, z_market: f64, z_sector: f64, z_idio: f64) -> f64 {
        self.sqrt_market * z_market + self.sqrt_sector * z_sector + self.sqrt_idio * z_idio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_satisfy_unit_variance() {
        let m = FactorModel::new(0.3, 0.4);
        let v = m.sqrt_market.powi(2) + m.sqrt_sector.powi(2) + m.sqrt_idio.powi(2);
        assert!((v - 1.0).abs() < 1e-12);
    }

    #[test]
    fn over_specified_correlations_get_normalised() {
        let m = FactorModel::new(0.7, 0.7);
        let v = m.sqrt_market.powi(2) + m.sqrt_sector.powi(2) + m.sqrt_idio.powi(2);
        assert!(v <= 1.0 + 1e-12);
    }
}

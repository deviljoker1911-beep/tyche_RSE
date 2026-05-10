//! Path-loss statistics.

/// Per-path realised loss.
pub type LossPath = f64;

/// Aggregate summary statistics extracted from a vector of path losses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LossSummary {
    /// Mean loss across paths.
    pub expected_loss: f64,
    /// 95th-percentile loss.
    pub var_95: f64,
    /// 99th-percentile loss.
    pub var_99: f64,
    /// Expected shortfall at the 97.5% level (mean of losses in the worst
    /// 2.5% tail).
    pub es_975: f64,
}

/// Compute summary statistics from a vector of per-path losses.
///
/// The input is mutated by sorting in place. This is intentional: callers
/// typically don't need the unsorted vector after summarising, and avoiding
/// the copy keeps the simulator's working set small.
#[must_use]
pub fn summarise(path_losses: &[LossPath]) -> LossSummary {
    if path_losses.is_empty() {
        return LossSummary {
            expected_loss: 0.0,
            var_95: 0.0,
            var_99: 0.0,
            es_975: 0.0,
        };
    }
    let mut sorted: Vec<f64> = path_losses.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let mean = sorted.iter().sum::<f64>() / n as f64;
    let var_95 = quantile(&sorted, 0.95);
    let var_99 = quantile(&sorted, 0.99);
    let tail_start = ((n as f64) * 0.975).ceil() as usize;
    let tail = &sorted[tail_start.min(n - 1)..];
    let es = if tail.is_empty() {
        0.0
    } else {
        tail.iter().sum::<f64>() / tail.len() as f64
    };
    LossSummary {
        expected_loss: mean,
        var_95,
        var_99,
        es_975: es,
    }
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let n = sorted.len();
    let idx = ((n as f64) * q).ceil() as usize;
    sorted[idx.saturating_sub(1).min(n - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles_make_sense() {
        let losses: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let s = summarise(&losses);
        assert_eq!(s.expected_loss, 50.5);
        assert!(s.var_95 >= 95.0);
        assert!(s.var_99 >= 99.0);
        assert!(s.es_975 >= s.var_95);
    }

    #[test]
    fn empty_input_returns_zeros() {
        let s = summarise(&[]);
        assert_eq!(s.expected_loss, 0.0);
        assert_eq!(s.var_95, 0.0);
    }
}

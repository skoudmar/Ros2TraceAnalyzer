use crate::statistics::Sorted;
use crate::utils::DurationDisplayImprecise;

pub struct DisplayDurationStats<'a>(pub &'a [i64]);

impl<'a> std::fmt::Display for DisplayDurationStats<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const QUANTILES: [f64; 5] = [0.0, 0.1, 0.5, 0.9, 1.0];
        if self.0.is_empty() {
            return write!(f, "No data");
        }

        let sorted = Sorted::from_unsorted(self.0);
        let quantiles = QUANTILES.map(|q| sorted.quantile(q).unwrap());
        write!(
            f,
            "min={}, 10th={}, median={}, 90th={}, max={}",
            DurationDisplayImprecise(*quantiles[0]),
            DurationDisplayImprecise(*quantiles[1]),
            DurationDisplayImprecise(*quantiles[2]),
            DurationDisplayImprecise(*quantiles[3]),
            DurationDisplayImprecise(*quantiles[4]),
        )
    }
}

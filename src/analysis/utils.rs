use crate::args::ANALYSIS_CLI_ARGS;
use crate::statistics::Sorted;
use crate::utils::DurationDisplayImprecise;

pub struct DisplayDurationStats<'a>(&'a [i64], &'a str);

impl<'a> DisplayDurationStats<'a> {
    pub fn with_newline(slice: &'a [i64]) -> Self {
        Self(slice, "\n")
    }

    pub fn with_comma(slice: &'a [i64]) -> Self {
        Self(slice, ", ")
    }

    pub fn new(slice: &'a [i64], separator: &'a str) -> Self {
        Self(slice, separator)
    }

    pub(crate) fn print(&self) {
        println!("{self}");
    }

    pub fn mean_and_std_dev(&self) -> (i64, f64) {
        let mean =
            (self.0.iter().copied().map(i128::from).sum::<i128>() / self.0.len() as i128) as i64;
        if self.0.len() == 1 {
            return (mean, f64::NAN);
        }
        let variance = self
            .0
            .iter()
            .map(|&x| (x - mean))
            .map(|x| i128::from(x) * i128::from(x))
            .sum::<i128>()
            / (self.0.len() - 1) as i128;
        let std_dev = (variance as f64).sqrt();
        (mean, std_dev)
    }
}

impl std::fmt::Display for DisplayDurationStats<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            return write!(f, "No data");
        }

        let sorted = Sorted::from_unsorted(self.0);
        write!(f, "count={}", sorted.len())?;
        for q in &ANALYSIS_CLI_ARGS
            .get()
            .expect("CLI arguments should be set")
            .quantiles
            .quantiles
        {
            let quantile = *sorted.quantile(*q).unwrap();
            write!(f, "{}{}={}", self.1, q, DurationDisplayImprecise(quantile))?;
        }
        Ok(())
    }
}

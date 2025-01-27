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
}

impl<'a> std::fmt::Display for DisplayDurationStats<'a> {
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

use crate::utils::DurationDisplayImprecise;

pub fn calculate_min_max_avg(elements: &[i64]) -> Option<(i64, i64, i64)> {
    let first = *elements.first()?;
    let mut min = first;
    let mut max = first;
    let mut sum = i128::from(first);

    for &element in elements.iter().skip(1) {
        if element < min {
            min = element;
        } else if element > max {
            max = element;
        }

        sum += i128::from(element);
    }

    let avg = (sum / elements.len() as i128)
        .try_into()
        .expect("Average of i64 values should fit into i64");

    Some((min, max, avg))
}

pub struct DisplayDurationStats<'a>(pub &'a [i64]);

impl<'a> std::fmt::Display for DisplayDurationStats<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.0.len();
        if let Some((min, max, avg)) = calculate_min_max_avg(self.0) {
            write!(
                f,
                "count={}, avg={}, min={}, max={}",
                count,
                DurationDisplayImprecise(avg),
                DurationDisplayImprecise(min),
                DurationDisplayImprecise(max),
            )
        } else {
            write!(f, "No data")
        }
    }
}

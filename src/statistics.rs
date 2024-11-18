use std::ops::Deref;

pub struct Sorted<T> {
    values: Vec<T>,
}

impl<T: Ord + Clone> Sorted<T> {
    #[must_use]
    pub fn from_unsorted(values: &[T]) -> Self {
        let mut values = values.to_vec();
        values.sort();
        Self { values }
    }
}

impl<T: Ord> Sorted<T> {
    /// Creates a new `Sorted` from a sorted `values` vector
    ///
    /// # Errors
    /// If the input vector is not sorted in ascending order,
    /// the function will return it back as `Err(values)`.
    pub fn from_sorted(values: Vec<T>) -> Result<Self, Vec<T>> {
        if values.is_sorted() {
            Ok(Self { values })
        } else {
            Err(values)
        }
    }
}

impl<T> Sorted<T> {
    /// Returns the value at the given quantile
    ///
    /// The quantile is a value between 0 and 1
    ///
    /// Uses the nearest rank method to calculate the quantile
    /// <https://en.wikipedia.org/wiki/Percentile#The_nearest-rank_method>
    ///
    /// # Errors
    /// If the input vector is empty, the function will return `None`
    ///
    /// # Panics
    /// If the quantile is not in range `0.0..=1.0`
    #[must_use]
    pub fn quantile(&self, quantile: f64) -> Option<&T> {
        assert!((0.0..=1.0).contains(&quantile));
        if self.values.is_empty() {
            return None;
        }
        let len = self.values.len() as f64;

        let index = (quantile * len).ceil() as usize;
        if let Some(index) = index.checked_sub(1) {
            self.values.get(index)
        } else {
            self.values.first()
        }
    }
}

impl<T> Deref for Sorted<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

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

#[cfg(test)]
mod test_sorted {
    use super::*;

    #[test]
    fn test_sorted_from_sorted() {
        let sorted = Sorted::from_sorted(vec![1, 2, 3, 4, 5]);

        assert!(sorted.is_ok());
    }

    #[test]
    fn test_sorted_from_unsorted() {
        let sorted = Sorted::from_unsorted(&[1, 2, 3, 4, 5]);

        assert_eq!(&[1, 2, 3, 4, 5], &*sorted);
    }

    #[test]
    fn test_unsorted_from_sorted() {
        let sorted = Sorted::from_sorted(vec![1, 0, 2]);
        assert!(sorted.is_err());
    }

    #[test]
    fn test_unsorted_from_unsorted() {
        let sorted = Sorted::from_unsorted(&[3, 1, 2]);
        assert_eq!(&[1, 2, 3], &*sorted);
    }

    #[test]
    fn test_empty_from_unsorted() {
        let sorted = Sorted::<i32>::from_unsorted(&[]);
        assert_eq!(None, sorted.quantile(0.0));
        assert_eq!(None, sorted.quantile(0.5));
        assert_eq!(None, sorted.quantile(1.0));
    }

    #[test]
    fn test_quantile() {
        let sorted = Sorted::from_unsorted(&[1, 2, 3, 4, 5]);
        assert_eq!(Some(&1), sorted.quantile(0.0));
        assert_eq!(Some(&1), sorted.quantile(0.1));
        assert_eq!(Some(&1), sorted.quantile(0.2));
        assert_eq!(Some(&2), sorted.quantile(0.3));
        assert_eq!(Some(&2), sorted.quantile(0.4));
        assert_eq!(Some(&3), sorted.quantile(0.5));
        assert_eq!(Some(&3), sorted.quantile(0.6));
        assert_eq!(Some(&4), sorted.quantile(0.7));
        assert_eq!(Some(&4), sorted.quantile(0.8));
        assert_eq!(Some(&5), sorted.quantile(0.9));
        assert_eq!(Some(&5), sorted.quantile(1.0));
    }
}

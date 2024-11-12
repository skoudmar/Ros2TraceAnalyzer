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

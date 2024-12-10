use std::collections::{HashMap, HashSet};

use crate::model::display::DisplayCallbackSummary;
use crate::model::{Callback, CallbackType};
use crate::statistics::{Mean, Quantile, Sorted};

use super::{ArcMutWrapper, CallbackDuration};

pub struct Utilization<'a> {
    callback_analysis: &'a CallbackDuration,
}

trait ReductionFunction: Fn(HashMap<u32, (usize, i64)>, Vec<i64>, f64) -> HashMap<u32, f64> {}

impl<F> ReductionFunction for F where
    F: Fn(HashMap<u32, (usize, i64)>, Vec<i64>, f64) -> HashMap<u32, f64>
{
}

impl<'a> Utilization<'a> {
    pub fn new(callback_analysis: &'a CallbackDuration) -> Self {
        Self { callback_analysis }
    }

    fn get_thread_callback_map(&self) -> HashMap<(String, u32), HashSet<ArcMutWrapper<Callback>>> {
        let mut thread_callback_map: HashMap<(String, u32), HashSet<ArcMutWrapper<Callback>>> =
            HashMap::new();
        for (callback_arc, execution_data) in self.callback_analysis.get_execution_data() {
            execution_data
                .iter()
                .map(|data| data.tid)
                .for_each(|thread| {
                    let callback = callback_arc.0.lock().unwrap();
                    let hostname = callback.get_hostname().to_string();
                    thread_callback_map
                        .entry((hostname.clone(), thread))
                        .or_default()
                        .insert(callback_arc.clone());
                });
        }

        thread_callback_map
    }

    fn calculate_utilization_per_callback_internal(
        &self,
        reduction_function: impl ReductionFunction,
    ) -> HashMap<ArcMutWrapper<Callback>, HashMap<u32, f64>> {
        let mut thread_utilization = HashMap::new();
        for (callback_arc, execution_data) in self.callback_analysis.get_execution_data() {
            let callback = callback_arc.0.lock().unwrap();
            let callback_is_timer = callback
                .get_type()
                .map_or(false, |t| t == CallbackType::Timer);

            let inter_arrival_time = callback_is_timer.then_some(()).and_then(|()| {
                callback
                    .get_caller()
                    .unwrap()
                    .unwrap_timer_ref()
                    .get_arc()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .get_period()
                    .into()
            });

            let Some(inter_arrival_time) = inter_arrival_time.map_or_else(
                || {
                    let ict = self
                        .callback_analysis
                        .get_inter_arrival_time(callback_arc)?;
                    ict.as_slice().mean()
                },
                Some,
            ) else {
                // Skip this callback if it is not a timer with known period or if the inter-arrival time is not known.
                continue;
            };

            let mut execution_times_and_counts_per_thread = HashMap::new();

            for data in execution_data {
                let thread = data.tid;
                let execution_time = data.duration;

                let execution_stats = execution_times_and_counts_per_thread
                    .entry(thread)
                    .or_insert_with(|| (0, 0));

                execution_stats.0 += 1; // Count
                execution_stats.1 += execution_time; // Time
            }

            let utilization_per_thread = reduction_function(
                execution_times_and_counts_per_thread,
                self.callback_analysis
                    .get_durations_for_callback(callback_arc)
                    .unwrap(),
                inter_arrival_time as f64,
            );

            thread_utilization.insert(callback_arc.clone(), utilization_per_thread);
        }

        thread_utilization
    }

    pub fn calculate_utilization_per_callback_real(
        &self,
    ) -> HashMap<ArcMutWrapper<Callback>, HashMap<u32, f64>> {
        self.calculate_utilization_per_callback_internal(
            |execution_times_and_counts_per_thread, duration_data, inter_arrival_time| {
                let total_count = duration_data.len();
                execution_times_and_counts_per_thread
                    .into_iter()
                    .map(|(thread, (_count, time))| {
                        let utilization = (time as f64) / (inter_arrival_time * total_count as f64);
                        (thread, utilization)
                    })
                    .collect()
            },
        )
    }

    pub fn calculate_utilization_per_callback(
        &self,
        execution_duration_quantile: Quantile,
    ) -> HashMap<ArcMutWrapper<Callback>, HashMap<u32, f64>> {
        self.calculate_utilization_per_callback_internal(
            |execution_times_and_counts_per_thread, duration_data, inter_arrival_time| {
                let total_count = duration_data.len();
                let quantile_duration = *Sorted::from(duration_data)
                    .quantile(execution_duration_quantile)
                    .unwrap();
                execution_times_and_counts_per_thread
                    .into_iter()
                    .map(|(thread, (count, _time))| {
                        let utilization = (quantile_duration as f64 * count as f64)
                            / (inter_arrival_time * total_count as f64);
                        (thread, utilization)
                    })
                    .collect()
            },
        )
    }

    pub fn calculate_total_utilization(
        thread_utilization_per_callback: &HashMap<ArcMutWrapper<Callback>, HashMap<u32, f64>>,
    ) -> HashMap<(String, u32), f64> {
        let mut utilization_per_thread_map = HashMap::new();

        for (callback_arc, utilization_per_thread) in thread_utilization_per_callback {
            let callback = callback_arc.0.lock().unwrap();
            let hostname = callback.get_hostname().to_string();

            for (thread, utilization) in utilization_per_thread {
                let thread_utilization = utilization_per_thread_map
                    .entry((hostname.clone(), *thread))
                    .or_insert_with(|| 0.0);
                *thread_utilization += utilization;
            }
        }

        utilization_per_thread_map
    }

    pub fn print_stats(&self, quantile: Quantile) {
        let thread_callback_map = self.get_thread_callback_map();
        let per_callback_utilization = self.calculate_utilization_per_callback(quantile);
        let utilization_per_thread = Self::calculate_total_utilization(&per_callback_utilization);
        let mut utilization_per_thread: Vec<_> = utilization_per_thread.into_iter().collect();
        utilization_per_thread.sort_unstable_by(|a, b| a.1.total_cmp(&b.1).reverse());

        println!("Utilization statistics for duration quantile {quantile}:");
        Self::print_utilization(
            &utilization_per_thread,
            &per_callback_utilization,
            &thread_callback_map,
        );
    }

    pub fn print_stats_real(&self) {
        let thread_callback_map = self.get_thread_callback_map();
        let per_callback_utilization = self.calculate_utilization_per_callback_real();
        let utilization_per_thread = Self::calculate_total_utilization(&per_callback_utilization);
        let mut utilization_per_thread: Vec<_> = utilization_per_thread.into_iter().collect();
        utilization_per_thread.sort_unstable_by(|a, b| a.1.total_cmp(&b.1).reverse());

        println!("Utilization statistics for real execution times:");
        Self::print_utilization(
            &utilization_per_thread,
            &per_callback_utilization,
            &thread_callback_map,
        );
    }

    fn print_utilization(
        utilization_per_thread: &[((String, u32), f64)],
        per_callback_utilization: &HashMap<ArcMutWrapper<Callback>, HashMap<u32, f64>>,
        thread_callback_map: &HashMap<(String, u32), HashSet<ArcMutWrapper<Callback>>>,
    ) {
        for (key @ (hostname, thread), utilization) in utilization_per_thread {
            let callbacks = thread_callback_map.get(key).unwrap();
            println!(
                "Thread {:?} on {} has utilization {:8.5} %",
                thread,
                hostname,
                utilization * 100.0,
            );
            for callback_arc in callbacks {
                let callback = callback_arc.0.lock().unwrap();
                let Some(per_callback_utilization) = per_callback_utilization.get(callback_arc)
                else {
                    continue;
                };
                let utilization = per_callback_utilization.get(thread).copied().unwrap_or(0.0);
                println!(
                    "  - {:8.5} % from Callback {}",
                    utilization * 100.0,
                    DisplayCallbackSummary(&callback),
                );
            }
        }
    }
}

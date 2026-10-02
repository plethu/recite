use super::TimingSummary;
use std::time::Duration;

impl TimingSummary {
    pub(super) fn from_durations(durations: Vec<Duration>) -> Self {
        let mut samples_ns = durations
            .into_iter()
            .map(|duration| duration.as_nanos())
            .collect::<Vec<_>>();
        samples_ns.sort_unstable();
        Self::from_sorted_samples(samples_ns)
    }

    #[must_use]
    pub fn from_samples(mut samples_ns: Vec<u128>) -> Self {
        samples_ns.sort_unstable();
        Self::from_sorted_samples(samples_ns)
    }

    fn from_sorted_samples(samples_ns: Vec<u128>) -> Self {
        let len = samples_ns.len();
        let min_ns = samples_ns.first().copied().unwrap_or(0);
        let max_ns = samples_ns.last().copied().unwrap_or(0);
        let median_ns = samples_ns.get(len / 2).copied().unwrap_or(0);
        let mean_ns = if len == 0 {
            0
        } else {
            samples_ns.iter().sum::<u128>() / len as u128
        };
        Self {
            samples_ns,
            min_ns,
            median_ns,
            mean_ns,
            max_ns,
        }
    }
}

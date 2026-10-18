//! DHAT's global peak includes setup. Filtered allocation-site totals at that timestamp
//! can be zero even for allocating work; compare the unfiltered process peaks instead.
use gungraun_summary::v7::BenchmarkSummary;
use std::{io::BufRead, path::Path};

type Error = Box<dyn std::error::Error>;

pub fn check_heap_peaks(reader: impl BufRead, profiles: &Path) -> Result<(), Error> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let summary: BenchmarkSummary = serde_json::from_str(&line)?;
        let benchmark = summary
            .benchmark_file
            .file_stem()
            .ok_or("benchmark file missing")?;
        let id = summary.id.as_deref().ok_or("benchmark identity missing")?;
        let name = format!("{}.{id}", summary.function_name);
        let directory = profiles.join(benchmark).join(&summary.group).join(&name);
        let candidate = peak(&std::fs::read_to_string(
            directory.join(format!("dhat.{name}.log")),
        )?)?;
        let control = peak(&std::fs::read_to_string(
            directory.join(format!("dhat.{name}.log.base@control")),
        )?)?;
        // Keep the existing 10% peak budget, using integer arithmetic at the boundary.
        if u128::from(candidate) * 100 > u128::from(control) * 110 {
            return Err(format!("{name}: whole-process peak heap regressed: {control} -> {candidate} bytes (limit 10%)").into());
        }
    }
    Ok(())
}

fn peak(log: &str) -> Result<u64, Error> {
    let mut peaks = log
        .lines()
        .filter_map(|line| line.split_once("At t-gmax:").map(|(_, rest)| rest));
    let value = peaks.next().ok_or("DHAT global peak missing")?;
    if peaks.next().is_some() {
        return Err("duplicate DHAT global peaks".into());
    }
    let (bytes, _) = value
        .trim()
        .split_once(" bytes in ")
        .ok_or("invalid DHAT global peak")?;
    let bytes = bytes.replace(',', "").parse::<u64>()?;
    if bytes == 0 {
        return Err("DHAT process peak is empty".into());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;

//! Coverage checks around the upstream Gungraun measurement/report owner.
use std::collections::BTreeSet;
use std::io::BufRead;

use gungraun_summary::either_or_both::EitherOrBoth;
use gungraun_summary::v7::{BenchmarkSummary, DhatMetric, EventKind, Metric, ToolMetricResults};

#[derive(Clone, Copy, Debug)]
pub enum Scope {
    Pr,
    Full,
    Writer,
    BoundText,
    Authoring,
}

fn expected(scope: Scope) -> BTreeSet<String> {
    if matches!(scope, Scope::Authoring) {
        return [
            "json_schema",
            "toml_schema",
            "schema_workspace_fork",
            "lsp_schema_actions",
            "lsp_change_saved_schema",
            "lsp_change_open_schema",
        ]
        .into_iter()
        .flat_map(|operation| {
            ["declarations_100", "declarations_1000"].map(|shape| format!("{operation}/{shape}"))
        })
        .chain(
            [
                "lsp_completion",
                "lsp_definition",
                "lsp_references",
                "lsp_publication",
            ]
            .into_iter()
            .flat_map(|operation| ["wide", "dense"].map(|shape| format!("{operation}/{shape}"))),
        )
        .chain(["catalog_import/cold_100", "catalog_import/populated_10000"].map(str::to_owned))
        .chain(["ui_labels/static_1000", "ui_labels/parameters_1000"].map(str::to_owned))
        .collect();
    }
    if matches!(scope, Scope::BoundText) {
        return ["prompt/authored_64".to_owned()].into_iter().collect();
    }
    if matches!(scope, Scope::Writer) {
        return [
            "cold_script/effects_1000",
            "cold_script/effects_10000",
            "changed_draft/wide",
            "changed_draft/dense",
            "accepted_draft/wide",
            "accepted_draft/dense",
            "completion_replacement/wide",
            "completion_replacement/dense",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
    }
    let fixtures: &[&str] = if matches!(scope, Scope::Full) {
        &["small", "realistic", "medium"]
    } else {
        &["small", "realistic"]
    };
    [
        "compile",
        "encode",
        "decode",
        "next_line",
        "localised_next",
        "choose",
        "traverse",
        "preview_recovery",
    ]
    .into_iter()
    .flat_map(|function| {
        fixtures
            .iter()
            .map(move |fixture| format!("{function}/{fixture}"))
    })
    .chain(std::iter::once("adapter_choice/sources_400".to_owned()))
    .collect()
}

/// Reject missing/duplicate workloads, absent controls, missing counters and regressions.
///
/// Gungraun intentionally creates a baseline when a requested one is missing. CI must instead
/// require comparisons, including for a newly added workload or renamed benchmark.
pub fn check_summaries(
    reader: impl BufRead,
    scope: Scope,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut seen = BTreeSet::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let summary: BenchmarkSummary = serde_json::from_str(&line)?;
        if summary.version != "7" || summary.baselines.1.as_deref() != Some("control") {
            return Err("comparison schema or named control differs".into());
        }
        let id = summary
            .id
            .as_deref()
            .ok_or("benchmark identity is missing")?;
        let key = format!("{}/{id}", summary.function_name);
        if !seen.insert(key.clone()) {
            return Err(format!("duplicate workload: {key}").into());
        }
        check_metrics(&summary).map_err(|error| format!("{key}: {error}"))?;
    }
    let expected = expected(scope);
    if seen != expected {
        return Err(format!(
            "workload coverage differs: missing {:?}; unexpected {:?}",
            expected.difference(&seen).collect::<Vec<_>>(),
            seen.difference(&expected).collect::<Vec<_>>()
        )
        .into());
    }
    Ok(())
}

fn paired(values: &EitherOrBoth<Metric>) -> Result<(u64, u64), Box<dyn std::error::Error>> {
    match values {
        EitherOrBoth::Both(Metric::Int(new), Metric::Int(old)) => Ok((*new, *old)),
        _ => Err("counter is missing its control or candidate integer measurement".into()),
    }
}

fn check_metrics(summary: &BenchmarkSummary) -> Result<(), Box<dyn std::error::Error>> {
    let mut instructions = false;
    let mut heap = false;
    for profile in &summary.profiles.0 {
        let total = &profile.data.total;
        if !total.regressions.is_empty() {
            return Err(format!(
                "regression in {}: {:?}",
                summary.function_name, total.regressions
            )
            .into());
        }
        match &total.metrics {
            ToolMetricResults::Callgrind(metrics) => {
                if instructions {
                    return Err("duplicate instruction profile".into());
                }
                let (new, old) = paired(
                    &metrics
                        .0
                        .get(&EventKind::Ir)
                        .ok_or("instruction counter missing")?
                        .values,
                )?;
                if new == 0 || old == 0 {
                    return Err("instruction measurement is empty".into());
                }
                instructions = true;
            }
            ToolMetricResults::Dhat(metrics) => {
                if heap {
                    return Err("duplicate heap profile".into());
                }
                for metric in [
                    DhatMetric::TotalBytes,
                    DhatMetric::TotalBlocks,
                    DhatMetric::AtTGmaxBytes,
                ] {
                    paired(&metrics.0.get(&metric).ok_or("heap counter missing")?.values)?;
                }
                heap = true;
            }
            _ => return Err("unexpected profiling tool".into()),
        }
    }
    if !instructions || !heap {
        return Err("instruction or heap profile missing".into());
    }
    Ok(())
}

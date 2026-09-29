use std::{
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use recite_benchmarks::report::{
    BenchGroup, BenchReport, BenchReportOptions, BenchTarget, build_bench_report, default_scale,
};
use recite_benchmarks::{BenchmarkFixture, BenchmarkScale};

use crate::{
    args::{BenchArgs, BenchFormat},
    error::CliError,
    fs::write_staged,
};

pub(super) fn bench_command(args: BenchArgs, stdout: &mut dyn Write) -> Result<(), CliError> {
    let target = bench_target(&args.target, &args.scale)?;
    let groups = bench_groups_for_target(&target, &args.group)?;
    let mut options = BenchReportOptions::new(target)
        .with_groups(groups)
        .with_samples(args.samples);
    if let Some(baseline) = &args.baseline {
        let source = std::fs::read_to_string(baseline).map_err(|source| CliError::Read {
            path: baseline.clone(),
            source,
        })?;
        let report = serde_json::from_str::<BenchReport>(&source).map_err(CliError::BenchJson)?;
        options = options.with_baseline(report);
    }
    let report = build_bench_report(&options)?;
    let rendered = match args.format {
        BenchFormat::Json => {
            let mut json = serde_json::to_string_pretty(&report).map_err(CliError::BenchJson)?;
            json.push('\n');
            json
        }
        BenchFormat::Markdown => report.to_markdown(),
    };

    if let Some(output) = args.output {
        write_staged(&output, rendered.as_bytes())?;
    } else {
        stdout.write_all(rendered.as_bytes())?;
    }
    Ok(())
}

fn bench_target(target: &str, scales: &[String]) -> Result<BenchTarget, CliError> {
    let path = Path::new(target);
    if path.is_dir() {
        if !scales.is_empty() {
            return Err(CliError::Bench {
                message: "--scale is only supported for synthetic fixture targets".to_owned(),
            });
        }
        return Ok(BenchTarget::ProjectRoot(PathBuf::from(path)));
    }

    if target == "synthetic" || target == "fixtures" {
        let selected_scales = parse_bench_scales(scales)?;
        return Ok(BenchTarget::Fixtures(
            selected_scales
                .into_iter()
                .map(BenchmarkFixture::Synthetic)
                .collect(),
        ));
    }

    let fixture = BenchmarkFixture::from_str(target)?;
    if !scales.is_empty() {
        return Err(CliError::Bench {
            message: "--scale cannot be combined with an explicit fixture id".to_owned(),
        });
    }
    Ok(BenchTarget::Fixtures(vec![fixture]))
}

fn parse_bench_scales(scales: &[String]) -> Result<Vec<BenchmarkScale>, CliError> {
    if scales.is_empty() {
        return Ok(vec![default_scale()]);
    }
    let mut selected = Vec::new();
    for scale in scales {
        let scale = BenchmarkScale::from_str(scale)?;
        if !selected.contains(&scale) {
            selected.push(scale);
        }
    }
    Ok(selected)
}

fn bench_groups(groups: &[String]) -> Result<Vec<BenchGroup>, CliError> {
    if groups.is_empty() {
        return Ok(BenchGroup::all().to_vec());
    }
    let mut selected = Vec::new();
    for group in groups {
        let group = BenchGroup::from_str(group)?;
        if !selected.contains(&group) {
            selected.push(group);
        }
    }
    Ok(selected)
}

fn bench_groups_for_target(
    target: &BenchTarget,
    groups: &[String],
) -> Result<Vec<BenchGroup>, CliError> {
    if groups.is_empty() && matches!(target, BenchTarget::ProjectRoot(_)) {
        return Ok(vec![BenchGroup::Compiler]);
    }
    bench_groups(groups)
}

#![cfg(test)]
//! Run with `cargo test -p recite-cli --test watch_stress -- --ignored --nocapture`.
//! `RECITE_BENCH_SCALES` selects generated fixture scales; the default is tiny and small.

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::{BenchmarkResult, BenchmarkScale};
use recite_core::{CompiledDialogue, SchemaFingerprint, decode_compiled_dialogue_messagepack};
use serde::Serialize;
use tempfile::{NamedTempFile, TempDir};

const WATCH_SUCCESS: &str = "watch: build succeeded";
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const REBUILD_TIMEOUT: Duration = Duration::from_secs(120);

#[test]
#[ignore = "generated fixture watch stress check; run explicitly for issue #108"]
fn generated_fixture_watch_rebuild_stress() -> Result<(), Box<dyn Error>> {
    let reports = BenchmarkScale::selected_from_env()?
        .into_iter()
        .map(run_watch_stress)
        .collect::<Result<Vec<_>, _>>()?;

    println!("{}", serde_json::to_string_pretty(&reports)?);
    Ok(())
}

fn run_watch_stress(scale: BenchmarkScale) -> Result<WatchStressReport, Box<dyn Error>> {
    let project = BenchmarkProject::load(scale)?;
    let root = TempDir::new()?;
    copy_project(&project, root.path())?;

    let fixture_counts = &project.summary().counts;
    let counts = FixtureCountsReport {
        blocks: fixture_counts.blocks,
        lines: fixture_counts.lines,
        choices: fixture_counts.choices,
        localisable_entries: fixture_counts.localisable_entries,
        generated_words: fixture_counts.generated_words,
        shards: fixture_counts.shards,
    };
    let source_path = first_source_path(&project, root.path())?;
    let schema_path = root.path().join("schema/synthetic.schema.json");
    let manifest_path = root.path().join("recite.project.toml");
    let original_asset_path = root.path().join("build/synthetic.recitec");

    let mut watch = WatchProcess::spawn(root.path())?;
    watch.wait_for_successes(1, REBUILD_TIMEOUT)?;
    let initial_asset = decode_asset(&original_asset_path)?;

    let source_successes = watch.success_count()?;
    let source_started = measured_now();
    edit_source(&source_path)?;
    let source_elapsed = watch.wait_for_fresh_output(source_started, source_successes, || {
        decode_asset(&original_asset_path).is_ok_and(|asset| {
            asset
                .lines
                .iter()
                .any(|line| line.source_text == "watch stress source edit.")
        })
    })?;
    let source_asset = decode_asset(&original_asset_path)?;
    if source_asset.sources == initial_asset.sources {
        return Err(test_error(
            "source edit did not refresh compiled source fingerprints",
        ));
    }

    let schema_successes = watch.success_count()?;
    let schema_started = measured_now();
    edit_schema(&schema_path)?;
    let source_schema_fingerprint = source_asset.header.schema_fingerprint;
    let schema_elapsed = watch.wait_for_fresh_output(schema_started, schema_successes, || {
        decode_asset(&original_asset_path)
            .is_ok_and(|asset| asset.header.schema_fingerprint != source_schema_fingerprint)
    })?;
    let schema_asset = decode_asset(&original_asset_path)?;
    if matches!(
        schema_asset.header.schema_fingerprint,
        SchemaFingerprint::NoSchema
    ) {
        return Err(test_error(
            "schema edit produced an asset without a schema fingerprint",
        ));
    }

    let replacement_asset = format!("build/watch-{}.recitec", scale.as_str());
    let replacement_asset_path = root.path().join(&replacement_asset);
    let manifest_successes = watch.success_count()?;
    let manifest_started = measured_now();
    edit_manifest(&manifest_path, &replacement_asset)?;
    let manifest_elapsed =
        watch.wait_for_fresh_output(manifest_started, manifest_successes, || {
            decode_asset(&replacement_asset_path)
                .is_ok_and(|asset| asset.header.asset_id.as_str() == replacement_asset)
        })?;

    watch.stop()?;

    Ok(WatchStressReport {
        report_schema: "recite-watch-stress-v1",
        fixture: scale.as_str(),
        counts,
        operations: vec![
            OperationReport::new(
                "source",
                source_elapsed,
                "compiled line and source fingerprint changed",
            ),
            OperationReport::new(
                "schema",
                schema_elapsed,
                "compiled schema fingerprint changed",
            ),
            OperationReport::new(
                "project_manifest",
                manifest_elapsed,
                "new manifest asset target was built",
            ),
        ],
        caveats: [
            "timings are informational and are not release gates",
            "timings are machine-specific",
            "each measurement is a whole-project rebuild",
            "elapsed time includes filesystem notification and debounce latency",
        ],
    })
}

fn copy_project(project: &BenchmarkProject, destination: &Path) -> BenchmarkResult<()> {
    for file in &project.summary().files {
        let source = project.root().join(&file.path);
        let target = destination.join(&file.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
    }
    Ok(())
}

fn first_source_path(project: &BenchmarkProject, root: &Path) -> Result<PathBuf, Box<dyn Error>> {
    project
        .summary()
        .files
        .iter()
        .find(|file| file.path.ends_with(".recite"))
        .map(|file| root.join(&file.path))
        .ok_or_else(|| test_error("generated fixture has no Recite source file"))
}

fn edit_source(path: &Path) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(path)?;
    let mut replaced = false;
    let edited = source
        .lines()
        .map(|line| {
            if !replaced && line.trim_start().starts_with("line 00000 001 ") {
                replaced = true;
                format!(
                    "{}watch stress source edit.",
                    &line[..line.len() - line.trim_start().len()]
                )
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !replaced {
        return Err(test_error(
            "generated fixture lacks the expected source line",
        ));
    }
    fs::write(path, format!("{edited}\n"))?;
    Ok(())
}

fn edit_schema(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut schema: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let speakers = schema
        .get_mut("speakers")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| test_error("generated schema has no speakers object"))?;
    speakers.insert(
        "watch_probe".to_owned(),
        serde_json::json!({"display_name": "Watch Probe"}),
    );
    fs::write(path, serde_json::to_vec_pretty(&schema)?)?;
    Ok(())
}

fn edit_manifest(path: &Path, replacement_asset: &str) -> Result<(), Box<dyn Error>> {
    let manifest = fs::read_to_string(path)?;
    let original = "asset = \"build/synthetic.recitec\"";
    if !manifest.contains(original) {
        return Err(test_error(
            "generated manifest lacks the expected asset target",
        ));
    }
    fs::write(
        path,
        manifest.replace(original, &format!("asset = \"{replacement_asset}\"")),
    )?;
    Ok(())
}

fn decode_asset(path: &Path) -> Result<CompiledDialogue, Box<dyn Error>> {
    Ok(decode_compiled_dialogue_messagepack(&fs::read(path)?)?)
}

fn test_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}

#[allow(clippy::disallowed_methods)]
fn measured_now() -> Instant {
    Instant::now()
}

struct WatchProcess {
    child: Option<Child>,
    stderr: NamedTempFile,
}

impl WatchProcess {
    fn spawn(project_root: &Path) -> Result<Self, Box<dyn Error>> {
        let stderr = NamedTempFile::new()?;
        let child = Command::new(env!("CARGO_BIN_EXE_recite"))
            .arg("watch")
            .arg(project_root)
            .current_dir(project_root)
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr.reopen()?))
            .spawn()?;
        Ok(Self {
            child: Some(child),
            stderr,
        })
    }

    fn wait_for_successes(
        &mut self,
        expected: usize,
        timeout: Duration,
    ) -> Result<(), Box<dyn Error>> {
        let started = measured_now();
        loop {
            let log = self.log()?;
            if log.matches(WATCH_SUCCESS).count() >= expected {
                return Ok(());
            }
            self.ensure_running(&log)?;
            if started.elapsed() >= timeout {
                return Err(test_error(format!(
                    "timed out waiting for {expected} successful watch builds\nstderr:\n{log}"
                )));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn wait_for_fresh_output(
        &mut self,
        started: Instant,
        successes_before: usize,
        fresh: impl Fn() -> bool,
    ) -> Result<Duration, Box<dyn Error>> {
        loop {
            let log = self.log()?;
            if log.matches(WATCH_SUCCESS).count() > successes_before && fresh() {
                return Ok(started.elapsed());
            }
            self.ensure_running(&log)?;
            if started.elapsed() >= REBUILD_TIMEOUT {
                return Err(test_error(format!(
                    "timed out waiting for fresh watch output\nstderr:\n{log}"
                )));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn log(&self) -> Result<String, Box<dyn Error>> {
        Ok(fs::read_to_string(self.stderr.path())?)
    }

    fn success_count(&self) -> Result<usize, Box<dyn Error>> {
        Ok(self.log()?.matches(WATCH_SUCCESS).count())
    }

    fn ensure_running(&mut self, log: &str) -> Result<(), Box<dyn Error>> {
        let Some(child) = self.child.as_mut() else {
            return Err(test_error("watch process was already stopped"));
        };
        if let Some(status) = child.try_wait()? {
            return Err(test_error(format!(
                "watch process exited unexpectedly with {status}\nstderr:\n{log}"
            )));
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        if child.try_wait()?.is_none() {
            child.kill()?;
            child.wait()?;
        }
        Ok(())
    }
}

impl Drop for WatchProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(Serialize)]
struct WatchStressReport {
    report_schema: &'static str,
    fixture: &'static str,
    counts: FixtureCountsReport,
    operations: Vec<OperationReport>,
    caveats: [&'static str; 4],
}

#[derive(Serialize)]
struct FixtureCountsReport {
    blocks: u32,
    lines: u32,
    choices: u32,
    localisable_entries: u32,
    generated_words: u32,
    shards: u32,
}

#[derive(Serialize)]
struct OperationReport {
    input: &'static str,
    elapsed_ms: u128,
    freshness_proof: &'static str,
}

impl OperationReport {
    fn new(input: &'static str, elapsed: Duration, freshness_proof: &'static str) -> Self {
        Self {
            input,
            elapsed_ms: elapsed.as_millis(),
            freshness_proof,
        }
    }
}

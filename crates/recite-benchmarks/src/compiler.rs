use recite_compiler::{
    CompileInput, CompileOptions, CompiledAssetOutput, PotDocument, ValidationReport,
    compile_inputs_with_schema, extract_pot_with_schema, validate_source_files,
    validate_source_files_with_schema,
};
use recite_core::{
    CompiledAssetId, CompilerVersion, ProjectSchema, SourceFile, SourceMapId,
    load_schema_manifest_str,
};
use recite_parser::parse;

use crate::project::BenchmarkProject;
use crate::{BenchmarkResult, error};

#[derive(Clone, Debug)]
pub struct CompilerProject {
    inputs: Vec<CompileInput>,
    source_files: Vec<SourceFile>,
    schema: ProjectSchema,
    options: CompileOptions,
}

impl CompilerProject {
    pub fn load(project: &BenchmarkProject) -> BenchmarkResult<Self> {
        let inputs = project
            .source_files()?
            .into_iter()
            .map(|file| CompileInput::new(file.path, file.source))
            .collect::<Vec<_>>();
        let source_files = lower_sources(&inputs)?;
        let schema_file = project.schema_file()?;
        let schema_report = load_schema_manifest_str(schema_file.path, &schema_file.source);
        if !schema_report.diagnostics.is_empty() {
            return Err(error(format!(
                "schema fixture has {} diagnostics",
                schema_report.diagnostics.len()
            )));
        }
        let Some(schema) = schema_report.schema else {
            return Err(error("schema fixture did not produce a schema"));
        };
        let options = CompileOptions::new(
            CompilerVersion::new("benchmarks")?,
            CompiledAssetId::new(format!("synthetic-{}", project.scale().as_str()))?,
            SourceMapId::new(format!("synthetic-{}-source-map", project.scale().as_str()))?,
            schema.canonical_fingerprint(),
        );
        Ok(Self {
            inputs,
            source_files,
            schema,
            options,
        })
    }

    #[must_use]
    pub fn compile_inputs(&self) -> Vec<CompileInput> {
        self.inputs.clone()
    }

    #[must_use]
    pub fn source_files(&self) -> Vec<SourceFile> {
        self.source_files.clone()
    }

    #[must_use]
    pub fn schema(&self) -> &ProjectSchema {
        &self.schema
    }

    #[must_use]
    pub fn options(&self) -> CompileOptions {
        self.options.clone()
    }

    pub fn compile_with_schema(&self) -> BenchmarkResult<CompiledProject> {
        let report =
            compile_inputs_with_schema(self.compile_inputs(), self.options(), self.schema())?;
        if !report.is_ok() {
            return Err(error(format!(
                "compile fixture produced {} diagnostics",
                report.diagnostics.len()
            )));
        }
        let Some(asset) = report.asset else {
            return Err(error("compile fixture did not produce an asset"));
        };
        Ok(CompiledProject { asset })
    }
}

#[derive(Clone, Debug)]
pub struct CompiledProject {
    asset: CompiledAssetOutput,
}

impl CompiledProject {
    #[must_use]
    pub fn asset(&self) -> &CompiledAssetOutput {
        &self.asset
    }
}

pub fn parse_inputs(inputs: &[CompileInput]) -> BenchmarkResult<usize> {
    let mut diagnostics = 0;
    for input in inputs {
        let parse = parse(&input.path, &input.source);
        diagnostics += parse.diagnostics().len();
    }
    if diagnostics != 0 {
        return Err(error(format!(
            "source fixtures produced {diagnostics} parse diagnostics"
        )));
    }
    Ok(inputs.len())
}

pub fn lower_inputs(inputs: &[CompileInput]) -> BenchmarkResult<Vec<SourceFile>> {
    lower_sources(inputs)
}

pub fn validate_without_schema(source_files: &[SourceFile]) -> ValidationReport {
    validate_source_files(source_files)
}

pub fn validate_with_schema(
    source_files: &[SourceFile],
    schema: &ProjectSchema,
) -> ValidationReport {
    validate_source_files_with_schema(source_files, schema)
}

pub fn compile_with_schema(project: &CompilerProject) -> BenchmarkResult<CompiledProject> {
    project.compile_with_schema()
}

pub fn extract_pot(project: &CompilerProject) -> BenchmarkResult<PotDocument> {
    let report = extract_pot_with_schema(project.compile_inputs(), project.schema());
    if !report.is_ok() {
        return Err(error(format!(
            "POT extraction fixture produced {} diagnostics",
            report.diagnostics.len()
        )));
    }
    report
        .catalog
        .ok_or_else(|| error("POT extraction did not produce a catalog"))
}

fn lower_sources(inputs: &[CompileInput]) -> BenchmarkResult<Vec<SourceFile>> {
    let mut source_files = Vec::with_capacity(inputs.len());
    for input in inputs {
        let parse = parse(&input.path, &input.source);
        let lowered = parse.lower_source_file();
        if !lowered.diagnostics.is_empty() {
            return Err(error(format!(
                "source fixture `{}` produced {} parse diagnostics",
                input.path,
                lowered.diagnostics.len()
            )));
        }
        source_files.push(lowered.source_file);
    }
    Ok(source_files)
}

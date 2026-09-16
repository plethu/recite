//! Original, schema-checked scenes for exercising everyday writer workflows.
use crate::{Document, EditError, ProjectContext, Workbench, WorkbenchError};
use recite_core::{DocumentKey, load_schema_manifest_str};

pub struct WriterExample {
    pub name: &'static str,
    pub source: &'static str,
    pub description: &'static str,
}

pub const WRITER_EXAMPLES: &[WriterExample] = &[
    WriterExample {
        name: "relay_hub.recite",
        source: include_str!(
            "../../../../../fixtures/recite/valid/writer_examples/relay_hub.recite"
        ),
        description: "Recurring hub · nested topics · quest and disposition effects",
    },
    WriterExample {
        name: "floodgate_waterfall.recite",
        source: include_str!(
            "../../../../../fixtures/recite/valid/writer_examples/floodgate_waterfall.recite"
        ),
        description: "Branch, converge, branch again · two endings",
    },
    WriterExample {
        name: "last_tram_linear.recite",
        source: include_str!(
            "../../../../../fixtures/recite/valid/writer_examples/last_tram_linear.recite"
        ),
        description: "Three sentences · no choices",
    },
];

impl WriterExample {
    pub fn open(&self) -> Result<Workbench, WorkbenchError> {
        let report = load_schema_manifest_str(
            "writer_examples.json",
            include_str!("../../../../../fixtures/schema/valid/writer_examples.json"),
        );
        let schema = report.schema.ok_or(WorkbenchError::ExampleSchema)?;
        let document = Document::in_project(
            DocumentKey::new(self.name).map_err(EditError::from)?,
            self.source,
            ProjectContext {
                documents: vec![],
                schema: Some(schema),
            },
        )?;
        Workbench::from_document(document)
    }
}

use std::collections::BTreeSet;

use serde_json::Value;

use crate::builder::{Builder, Target};
use crate::diagnostics::{INVALID, UNSUPPORTED};
use crate::{FieldMapping, ImportError, ImportRequest, Location, Provenance, SourceFamily};

mod json;

struct Field {
    name: String,
    value: Value,
    provenance: Provenance,
}

pub(super) fn read(request: &ImportRequest<'_>, builder: &mut Builder) -> Result<(), ImportError> {
    let root = builder.provenance(Location::Document, "document");
    let Some(mapping) = request.mapping else {
        return builder.issue(
            INVALID,
            root,
            "mapping",
            "JSON/CSV requires an explicit field mapping.",
        );
    };
    let names = field_names(mapping);
    if names.iter().any(|name| name.is_empty())
        || names.iter().collect::<BTreeSet<_>>().len() != names.len()
    {
        return builder.issue(
            INVALID,
            root,
            "mapping",
            "Mapped fields must be nonempty and distinct.",
        );
    }
    if request.family == SourceFamily::Json {
        read_json(request.source, mapping, builder)
    } else {
        read_csv(request.source, mapping, builder)
    }
}

fn read_json(
    source: &str,
    mapping: &FieldMapping,
    builder: &mut Builder,
) -> Result<(), ImportError> {
    let records = match serde_json::from_str::<Vec<json::UniqueRecord>>(source) {
        Ok(records) => records,
        Err(error) => {
            return builder.issue(
                INVALID,
                builder.provenance(Location::Document, "document"),
                "json",
                error.to_string(),
            );
        }
    };
    for (index, record) in records.into_iter().enumerate() {
        let fields = record
            .0
            .into_iter()
            .map(|(name, value)| {
                let pointer = format!("/{index}/{}", name.replace('~', "~0").replace('/', "~1"));
                let provenance = builder.provenance(Location::Json { pointer }, index.to_string());
                Field {
                    name,
                    value,
                    provenance,
                }
            })
            .collect::<Vec<_>>();
        let root = builder.provenance(
            Location::Json {
                pointer: format!("/{index}"),
            },
            index.to_string(),
        );
        convert(&fields, mapping, root, builder)?;
    }
    Ok(())
}

fn read_csv(
    source: &str,
    mapping: &FieldMapping,
    builder: &mut Builder,
) -> Result<(), ImportError> {
    let mut reader = csv::ReaderBuilder::new().from_reader(source.as_bytes());
    let headers = match reader.headers() {
        Ok(headers) => headers.clone(),
        Err(error) => {
            return builder.issue(
                INVALID,
                builder.provenance(Location::Document, "header"),
                "csv",
                error.to_string(),
            );
        }
    };
    if headers.is_empty()
        || headers.iter().any(str::is_empty)
        || headers.iter().collect::<BTreeSet<_>>().len() != headers.len()
    {
        return builder.issue(
            INVALID,
            builder.provenance(Location::Document, "header"),
            "csv",
            "CSV headers must be nonempty and distinct.",
        );
    }
    for (index, result) in reader.records().enumerate() {
        let row = index as u64 + 2;
        let record = match result {
            Ok(record) => record,
            Err(error) => {
                builder.issue(
                    INVALID,
                    builder.provenance(
                        Location::Csv {
                            row,
                            column: 1,
                            header: headers[0].to_owned(),
                        },
                        row.to_string(),
                    ),
                    "csv",
                    error.to_string(),
                )?;
                continue;
            }
        };
        let fields = headers
            .iter()
            .zip(record.iter())
            .enumerate()
            .map(|(column, (name, value))| Field {
                name: name.to_owned(),
                value: Value::String(value.to_owned()),
                provenance: builder.provenance(
                    Location::Csv {
                        row,
                        column: column + 1,
                        header: name.to_owned(),
                    },
                    row.to_string(),
                ),
            })
            .collect::<Vec<_>>();
        let root = fields[0].provenance.clone();
        convert(&fields, mapping, root, builder)?;
    }
    Ok(())
}

fn convert(
    fields: &[Field],
    mapping: &FieldMapping,
    root: Provenance,
    builder: &mut Builder,
) -> Result<(), ImportError> {
    let names = field_names(mapping);
    let mut valid = true;
    for field in fields {
        if !names.contains(&field.name.as_str()) {
            builder.issue(
                UNSUPPORTED,
                field.provenance.clone(),
                "field",
                format!("Unmapped field: {}", field.name),
            )?;
        } else if !field.value.is_string() {
            builder.issue(
                INVALID,
                field.provenance.clone(),
                "field",
                format!("Mapped field {} must be a string.", field.name),
            )?;
            valid = false;
        }
    }
    for name in &names {
        if !fields.iter().any(|field| field.name == *name) {
            builder.issue(
                INVALID,
                root.clone(),
                "field",
                format!("Missing mapped field: {name}"),
            )?;
            valid = false;
        }
    }
    if !valid {
        return Ok(());
    }
    let get = |name: &str| fields.iter().find(|field| field.name == name);
    let text = |name: &str| get(name).and_then(|field| field.value.as_str());
    let Some(block) = text(&mapping.block) else {
        return Ok(());
    };
    if builder.current_block() != Some(block) {
        builder.block(
            block,
            get(&mapping.block).map_or(root.clone(), |field| field.provenance.clone()),
        )?;
    }
    let Some(line) = text(&mapping.text) else {
        return Ok(());
    };
    let optional = |name: &Option<String>| {
        name.as_deref()
            .and_then(text)
            .filter(|value| !value.is_empty())
    };
    builder.line(
        line,
        optional(&mapping.speaker),
        optional(&mapping.id),
        get(&mapping.text).map_or(root.clone(), |field| field.provenance.clone()),
    )?;
    if let Some(target) = optional(&mapping.target) {
        builder.jump(
            if target == "END" {
                Target::End
            } else {
                Target::Block(target)
            },
            get(mapping.target.as_deref().unwrap_or(""))
                .map_or(root, |field| field.provenance.clone()),
        )?;
    }
    Ok(())
}

fn field_names(mapping: &FieldMapping) -> Vec<&str> {
    [
        &Some(mapping.block.as_str()),
        &Some(mapping.text.as_str()),
        &mapping.id.as_deref(),
        &mapping.speaker.as_deref(),
        &mapping.target.as_deref(),
    ]
    .into_iter()
    .filter_map(|name| *name)
    .collect()
}

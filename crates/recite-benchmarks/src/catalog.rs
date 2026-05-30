use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use recite_core::LocaleId;
use recite_runtime::{LocaleProvider, TextDomain};

use crate::fixture_context::RuntimeFixture;
use crate::project::BenchmarkProject;
use crate::{BenchmarkResult, error};

#[derive(Clone, Debug, Default)]
pub struct CatalogProvider {
    entries: BTreeMap<String, String>,
}

impl CatalogProvider {
    pub fn load(project: &BenchmarkProject, fixture: &RuntimeFixture) -> BenchmarkResult<Self> {
        let mut entries = BTreeMap::new();
        for paths in fixture.catalogs().values() {
            for path in paths {
                let source = fs::read_to_string(project.root().join(path))?;
                entries.extend(parse_po_catalog(&source)?);
            }
        }
        Ok(Self { entries })
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&str> {
        self.entries.get(id).map(String::as_str)
    }
}

impl LocaleProvider for CatalogProvider {
    fn lookup(
        &self,
        id: &str,
        _source_text: &str,
        _domain: TextDomain,
        _locale: &LocaleId,
        _variant: Option<&str>,
    ) -> Option<String> {
        self.entries.get(id).cloned()
    }
}

pub fn parse_po_catalog(source: &str) -> BenchmarkResult<BTreeMap<String, String>> {
    let mut entries = BTreeMap::new();
    let mut context = None::<String>;
    let mut message = None::<String>;

    for line in source.lines() {
        if let Some(value) = line.strip_prefix("msgctxt ") {
            context = Some(parse_po_string(value)?);
        } else if line.strip_prefix("msgid ").is_some() {
            message = Some(String::new());
        } else if let Some(value) = line.strip_prefix("msgstr ") {
            let Some(id) = context.take() else {
                return Err(error("PO msgstr appeared before msgctxt"));
            };
            let translated = parse_po_string(value)?;
            entries.insert(id, translated);
            message = None;
        } else if !line.trim().is_empty() && message.is_some() {
            return Err(error(format!("unsupported multi-line PO entry `{line}`")));
        }
    }

    Ok(entries)
}

fn parse_po_string(value: &str) -> BenchmarkResult<String> {
    let trimmed = value.trim();
    let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    else {
        return Err(error(format!("malformed PO string `{value}`")));
    };

    let mut output = String::new();
    let mut chars = inner.chars();
    while let Some(char) = chars.next() {
        if char == '\\' {
            let Some(escaped) = chars.next() else {
                return Err(error("PO string ends with a dangling escape"));
            };
            output.push(match escaped {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                other => other,
            });
        } else {
            output.push(char);
        }
    }
    Ok(output)
}

#[allow(dead_code)]
fn _assert_path(_: &Path) {}

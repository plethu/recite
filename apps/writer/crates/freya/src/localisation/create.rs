//! New catalogues use gettext metadata and Recite's lossless validation.
use super::target::TargetLocale;
use super::{
    catalogue::Catalogue,
    messages::{MsgId, text as wording},
};
use recite_core::po::{PoDiagnosticKind, PoDocument, PoEdit};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use wait_timeout::ChildExt;

pub(super) fn language(value: &str) -> Result<TargetLocale, String> {
    TargetLocale::parse(value)
}

pub(super) fn suggested_path(root: &Path, locale: &str) -> PathBuf {
    root.join("locale").join(format!("{locale}.po"))
}

pub(super) struct Preparation {
    result: mpsc::Receiver<Result<PoDocument, String>>,
    cancelled: Arc<AtomicBool>,
}
impl Drop for Preparation {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}
impl Preparation {
    pub fn start(template: String, locale: TargetLocale) -> Result<Self, String> {
        let (sender, result) = mpsc::sync_channel(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = cancelled.clone();
        std::thread::Builder::new()
            .name("recite-catalogue-create".into())
            .spawn(move || {
                let _ = sender.send(initialise(&template, &locale, &cancellation));
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { result, cancelled })
    }
    pub fn poll(&self) -> Option<Result<PoDocument, String>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Err(wording(MsgId::WriterCreationFailed)))
            }
        }
    }
}

fn initialise(
    template: &str,
    locale: &TargetLocale,
    cancelled: &AtomicBool,
) -> Result<PoDocument, String> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(wording(MsgId::WriterCreationFailed));
    }
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let input = directory.path().join("messages.pot");
    let output = directory.path().join("catalogue.po");
    // A deterministic neutral header; no translator identity or machine locale.
    let header = "msgid \"\"\nmsgstr \"\"\n\"MIME-Version: 1.0\\n\"\n\"Content-Type: text/plain; charset=UTF-8\\n\"\n\"Content-Transfer-Encoding: 8bit\\n\"\n\"Plural-Forms: nplurals=INTEGER; plural=EXPRESSION;\\n\"\n\n";
    let header = if template
        .lines()
        .any(|line| line.starts_with("msgid_plural "))
    {
        header.to_owned()
    } else {
        header.replace(
            "\"Plural-Forms: nplurals=INTEGER; plural=EXPRESSION;\\n\"\n",
            "",
        )
    };
    fs::write(&input, format!("{header}{template}")).map_err(|e| e.to_string())?;
    let errors = directory.path().join("errors.txt");
    let stderr = fs::File::create(&errors).map_err(|e| e.to_string())?;
    let mut child = Command::new("msginit")
        .arg("--no-translator")
        .arg("--no-wrap")
        .arg(format!(
            "--locale={}.UTF-8",
            locale.base().to_string().replace('-', "_")
        ))
        .arg("--input")
        .arg(&input)
        .arg("--output-file")
        .arg(&output)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                wording(MsgId::WriterGettextMissing)
            } else {
                e.to_string()
            }
        })?;
    match child.wait_timeout(Duration::from_secs(15)) {
        Ok(Some(status)) if status.success() && !cancelled.load(Ordering::Relaxed) => {}
        result => {
            // Cancellation discards the result; a hung initializer is killed and reaped.
            let _ = child.kill();
            let _ = child.wait();
            let detail = result.err().map_or_else(
                || fs::read_to_string(errors).unwrap_or_default(),
                |e| e.to_string(),
            );
            return Err(format!(
                "{}\n{detail}",
                wording(MsgId::WriterCreationFailed)
            ));
        }
    }
    let text = fs::read_to_string(output).map_err(|e| e.to_string())?;
    let document = PoDocument::parse(text).map_err(|error| match error.kind() {
        PoDiagnosticKind::InvalidHeader(_) | PoDiagnosticKind::InvalidPluralRule(_) => {
            wording(MsgId::WriterPluralUnknown)
        }
        _ => error.to_string(),
    })?;
    normalise(document, locale)
}

fn normalise(mut document: PoDocument, locale: &TargetLocale) -> Result<PoDocument, String> {
    let plural = document
        .headers()
        .iter()
        .find(|h| h.key().eq_ignore_ascii_case("Plural-Forms"));
    if document.entries().iter().any(|e| e.is_plural()) && plural.is_none() {
        return Err(wording(MsgId::WriterPluralUnknown));
    }
    // Keep only deliberate catalogue metadata, never inferred author identities.
    let mut header = format!(
        "Language: {locale}\nMIME-Version: 1.0\nContent-Type: text/plain; charset=UTF-8\nContent-Transfer-Encoding: 8bit\n"
    );
    if let Some(plural) = plural {
        recite_core::po::validate_plural_rule(plural.value()).map_err(|e| e.to_string())?;
        header.push_str(&format!("Plural-Forms: {}\n", plural.value()));
    }
    let mut edits = Vec::new();
    for entry in document.entries() {
        if entry.is_header() {
            edits.push(PoEdit::translation(entry.id(), &header));
        } else if entry.is_plural() {
            for arm in entry.plural_translations() {
                if let Some(index) = arm.index() {
                    edits.push(PoEdit::plural_translation(entry.id(), index, ""));
                }
            }
        } else {
            // msginit may fill source-language catalogues: translation starts blank.
            edits.push(PoEdit::translation(entry.id(), ""));
        }
    }
    document.apply_edits(edits).map_err(|e| e.to_string())?;
    Ok(document)
}

pub(super) fn persist(document: &PoDocument, path: &Path) -> Result<Catalogue, String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temporary
        .write_all(document.as_bytes())
        .map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    temporary.persist_noclobber(path).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            wording(MsgId::WriterCatalogueExists)
        } else {
            e.error.to_string()
        }
    })?;
    #[cfg(unix)]
    if let Ok(directory) = fs::File::open(parent) {
        let _ = directory.sync_all();
    }
    Catalogue::open_recoverable(path)
}

#[cfg(test)]
mod tests;

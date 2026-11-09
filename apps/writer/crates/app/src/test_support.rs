//! Deterministic project fixtures and worker drivers for Writer tests.
//!
//! Native callbacks schedule owned work; these helpers execute it synchronously so tests in this
//! crate and the frontend observe the same ordering without a toolkit.
pub use crate::Writer;
use crate::navigation::worker::Job as NavigationJob;
use recite_writer_session::ProjectSession;
use recite_writer_session::project::save::sequence;
use std::fs;
use tokio::sync::mpsc as async_mpsc;

pub fn project(source: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"test\"\nversion = \"1\"\n",
    )?;
    fs::write(directory.path().join("scene.recite"), source)?;
    Ok(directory)
}

pub fn open(directory: &tempfile::TempDir) -> Result<Writer, Box<dyn std::error::Error>> {
    let session =
        ProjectSession::open(directory.path(), Some(std::path::Path::new("scene.recite")))?;
    Ok(Writer::from_session(
        crate::session::Session::Project(Box::new(session)),
        None,
    ))
}

pub fn finish_navigation(
    writer: &mut Writer,
    requests: &mut async_mpsc::UnboundedReceiver<NavigationJob>,
) -> Result<(), Box<dyn std::error::Error>> {
    while writer.navigation.busy() {
        let completed = requests.try_recv()?.execute();
        let request = completed.request.clone();
        if let Some(retired) = writer.accept_navigation(completed) {
            retired.retire()?;
        }
        writer.navigation_retired(&request);
    }
    Ok(())
}

pub fn finish_work(
    writer: &mut Writer,
    requests: &mut async_mpsc::UnboundedReceiver<sequence::Work>,
) -> Result<(), Box<dyn std::error::Error>> {
    while writer.work.busy() {
        let work = requests.try_recv()?;
        writer.accept_save_work(work.execute());
    }
    Ok(())
}

pub fn save(writer: &mut Writer) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.save();
    finish_work(writer, &mut requests)
}

impl crate::shell::Shell {
    /// A shell already showing `writer`, as after a completed project opening.
    pub fn with_writer(writer: Writer) -> Self {
        Self {
            writer: Some(writer),
            show_welcome: false,
            status: String::new(),
            ..Self::empty()
        }
    }
}

/// Put a Writer into `phase` directly, as a frontend test scenario setup.
pub fn set_phase(writer: &mut Writer, phase: crate::Phase) {
    writer.phase = phase;
}
/// Put document actions into `menu` directly, as a frontend test scenario setup.
pub fn set_document_menu(writer: &mut Writer, menu: crate::attention::DocumentMenu) {
    writer.document_menu = menu;
}
/// Show or hide the project welcome overlay directly, as a frontend test scenario setup.
pub fn set_welcome(shell: &mut crate::shell::Shell, shown: bool) {
    shell.show_welcome = shown;
}
/// Show or hide settings directly, as a frontend test scenario setup.
pub fn set_settings(shell: &mut crate::shell::Shell, shown: bool) {
    shell.settings_open = shown;
}

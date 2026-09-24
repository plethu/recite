//! Linux writer ownership and bounded local-socket forwarding.
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use super::Activation;

const QUEUE_SIZE: usize = 16;
const IO_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_PAUSE: Duration = Duration::from_millis(25);

mod wire;
use wire::{monotonic_now, read_frame, write_frame};

#[derive(Serialize, Deserialize)]
struct Request {
    project: Option<PathBuf>,
    route: Option<String>,
}

#[derive(Serialize, Deserialize)]
enum Receipt {
    Queued,
    Applied,
    Refused(String),
}

pub struct IncomingRoute {
    pub(crate) project: Option<PathBuf>,
    pub(crate) route: Option<String>,
    pub(crate) cancelled: Arc<AtomicBool>,
    pub(crate) reply: mpsc::Sender<Result<(), String>>,
}

#[derive(Clone)]
pub struct ActivationInbox {
    receiver: Arc<Mutex<Receiver<IncomingRoute>>>,
    polling: Arc<AtomicBool>,
}

impl ActivationInbox {
    pub(crate) fn new(receiver: Receiver<IncomingRoute>) -> Self {
        Self {
            receiver: Arc::new(Mutex::new(receiver)),
            polling: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn try_next(&self) -> Option<IncomingRoute> {
        // Socket ownership precedes window creation. Do not queue requests
        // until the UI has started consuming them.
        self.polling.store(true, Ordering::Release);
        self.receiver.lock().ok()?.try_recv().ok()
    }
}

pub struct ActivationHost {
    _lock: File,
    socket: PathBuf,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    inbox: ActivationInbox,
}

impl ActivationHost {
    pub fn inbox(&self) -> ActivationInbox {
        self.inbox.clone()
    }
}

impl Drop for ActivationHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // The lock remains held until after this socket is removed.
        if fs::symlink_metadata(&self.socket).is_ok_and(|meta| meta.file_type().is_socket()) {
            let _ = fs::remove_file(&self.socket);
        }
    }
}

pub(super) fn claim(project: Option<&Path>, route: Option<&str>) -> Result<Activation, String> {
    let store = recite_config::UserConfigStore::discover().map_err(|e| e.to_string())?;
    // This is the parent used by UserConfigStore::state_file.
    store
        .state_file("writer-activation.state")
        .map_err(|e| e.to_string())?;
    let config_parent = store
        .path()
        .and_then(Path::parent)
        .ok_or("No user state directory is available")?;
    fs::create_dir_all(config_parent).map_err(|e| e.to_string())?;
    let parent_meta = fs::symlink_metadata(config_parent).map_err(|e| e.to_string())?;
    if !parent_meta.is_dir()
        || parent_meta.uid() != current_uid()?
        || parent_meta.mode() & 0o022 != 0
    {
        return Err("Unsafe user configuration directory".into());
    }
    let directory = config_parent.join("writer-activation");
    claim_at(&directory, project, route)
}

fn claim_at(
    directory: &Path,
    project: Option<&Path>,
    route: Option<&str>,
) -> Result<Activation, String> {
    private_directory(directory)?;
    let key = "writer";
    let lock_path = directory.join(format!("{key}.lock"));
    let socket = directory.join(format!("{key}.sock"));
    let lock = regular_lock_file(&lock_path)?;
    match lock.try_lock() {
        Ok(()) => owner(lock, socket),
        Err(fs::TryLockError::WouldBlock) => {
            forward(
                &socket,
                &Request {
                    project: project.map(Path::to_path_buf),
                    route: route.map(str::to_owned),
                },
            )?;
            Ok(Activation::Forwarded)
        }
        Err(fs::TryLockError::Error(error)) => {
            Err(format!("Could not lock writer ownership: {error}"))
        }
    }
}

fn owner(lock: File, socket: PathBuf) -> Result<Activation, String> {
    match fs::symlink_metadata(&socket) {
        Ok(meta) if meta.file_type().is_socket() && meta.uid() == current_uid()? => {
            fs::remove_file(&socket)
                .map_err(|e| format!("Could not reclaim old writer socket: {e}"))?;
        }
        Ok(_) => return Err("Unsafe writer socket entry".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Could not inspect writer socket: {error}")),
    }
    let listener =
        UnixListener::bind(&socket).map_err(|e| format!("Could not bind writer socket: {e}"))?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("Could not secure writer socket: {e}"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let (sender, receiver) = mpsc::sync_channel(QUEUE_SIZE);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = stop.clone();
    let inbox = ActivationInbox::new(receiver);
    let polling = inbox.polling.clone();
    let worker = thread::Builder::new()
        .name("recite-writer-activation".into())
        .spawn(move || serve(listener, sender, worker_stop, polling))
        .map_err(|e| format!("Could not start writer activation: {e}"))?;
    Ok(Activation::Owner(ActivationHost {
        _lock: lock,
        socket,
        stop,
        worker: Some(worker),
        inbox,
    }))
}

fn serve(
    listener: UnixListener,
    sender: SyncSender<IncomingRoute>,
    stop: Arc<AtomicBool>,
    polling: Arc<AtomicBool>,
) {
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = handle(&mut stream, &sender, &polling);
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => thread::sleep(ACCEPT_PAUSE),
            Err(error) => {
                eprintln!("Writer activation listener stopped: {error}");
                break;
            }
        }
    }
}

fn handle(
    stream: &mut UnixStream,
    sender: &SyncSender<IncomingRoute>,
    polling: &AtomicBool,
) -> Result<(), String> {
    let request: Request = match read_frame(stream) {
        Ok(request) => request,
        Err(error) => {
            let _ = write_frame(stream, &Receipt::Refused(error.clone()));
            return Err(error);
        }
    };
    if request
        .project
        .as_ref()
        .is_some_and(|path| !path.is_absolute())
    {
        write_frame(stream, &Receipt::Refused("Invalid project path".into()))?;
        return Ok(());
    }
    let project = match canonical_intent(&request) {
        Ok(project) => project,
        Err(error) => {
            write_frame(stream, &Receipt::Refused(error))?;
            return Ok(());
        }
    };
    if !polling.load(Ordering::Acquire) {
        write_frame(
            stream,
            &Receipt::Refused("The writer is still opening a project.".into()),
        )?;
        return Ok(());
    }
    let (reply, result) = mpsc::channel();
    let cancelled = Arc::new(AtomicBool::new(false));
    if sender
        .try_send(IncomingRoute {
            project,
            route: request.route,
            cancelled: cancelled.clone(),
            reply,
        })
        .is_err()
    {
        write_frame(
            stream,
            &Receipt::Refused("The writer is busy handling links".into()),
        )?;
        return Ok(());
    }
    if let Err(error) = write_frame(stream, &Receipt::Queued) {
        cancelled.store(true, Ordering::Release);
        return Err(error);
    }
    let receipt = match result.recv_timeout(IO_TIMEOUT) {
        Ok(Ok(())) => Receipt::Applied,
        Ok(Err(error)) => Receipt::Refused(error),
        Err(_) => {
            cancelled.store(true, Ordering::Release);
            Receipt::Refused(
                "The writer did not report an outcome before the deadline; the request was cancelled if still pending".into(),
            )
        }
    };
    write_frame(stream, &receipt)
}

fn canonical_intent(request: &Request) -> Result<Option<PathBuf>, String> {
    let linked = request
        .route
        .as_deref()
        .map(crate::navigation::project_from_route)
        .transpose()?
        .flatten();
    let canonical = |path: &Path| {
        recite_config::discover_project(path)
            .map(|report| report.manifest().project_root().to_owned())
            .map_err(|error| format!("Could not open linked project {}: {error}", path.display()))
    };
    match (request.project.as_deref(), linked.as_deref()) {
        (Some(project), Some(linked)) => {
            let project = canonical(project)?;
            if project != canonical(linked)? {
                return Err("The link identifies another project".into());
            }
            Ok(Some(project))
        }
        (Some(project), None) => canonical(project).map(Some),
        (None, Some(_)) => Err("A project link needs a project intent".into()),
        (None, None) => Ok(None),
    }
}

fn forward(socket: &Path, request: &Request) -> Result<(), String> {
    let deadline = monotonic_now() + IO_TIMEOUT;
    let mut stream = loop {
        match UnixStream::connect(socket) {
            Ok(stream) => break stream,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                ) && monotonic_now() < deadline =>
            {
                thread::sleep(ACCEPT_PAUSE)
            }
            Err(error) => return Err(format!("Could not contact the open writer: {error}")),
        }
    };
    stream
        .set_read_timeout(Some(IO_TIMEOUT))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(IO_TIMEOUT))
        .map_err(|e| e.to_string())?;
    write_frame(&mut stream, request)?;
    match read_frame::<Receipt>(&mut stream)? {
        Receipt::Queued => {}
        Receipt::Refused(error) => return Err(error),
        Receipt::Applied => return Err("Invalid writer acknowledgement".into()),
    }
    match read_frame::<Receipt>(&mut stream)? {
        Receipt::Applied => Ok(()),
        Receipt::Refused(error) => Err(error),
        Receipt::Queued => Err("Writer did not finish opening the link".into()),
    }
}

fn current_uid() -> Result<u32, String> {
    fs::metadata("/proc/self")
        .map(|meta| meta.uid())
        .map_err(|e| e.to_string())
}

fn private_directory(directory: &Path) -> Result<(), String> {
    if !directory.exists() {
        let mut builder = fs::DirBuilder::new();
        builder.mode(0o700).recursive(true);
        builder.create(directory).map_err(|e| e.to_string())?;
    }
    let meta = fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
    if !meta.is_dir() || meta.uid() != current_uid()? || meta.mode() & 0o077 != 0 {
        return Err("Unsafe writer activation directory".into());
    }
    Ok(())
}

fn regular_lock_file(path: &Path) -> Result<File, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() || meta.uid() != current_uid()? || meta.mode() & 0o077 != 0 => {
            return Err("Unsafe writer lock entry".into());
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .map_err(|e| e.to_string())?;
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.is_file() || meta.uid() != current_uid()? || meta.mode() & 0o077 != 0 {
        return Err("Unsafe writer lock entry".into());
    }
    Ok(file)
}

#[cfg(test)]
mod tests;

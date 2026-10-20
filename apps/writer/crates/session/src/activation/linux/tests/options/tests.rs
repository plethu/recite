use super::*;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

#[test]
fn forwarding_preserves_native_project_document_and_editor_arguments()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let project = dir
        .path()
        .join(std::ffi::OsString::from_vec(b"project-\xff".to_vec()));
    fs::create_dir(&project)?;
    fs::write(project.join("recite.project.toml"), "format_version = 1\n")?;
    fs::write(project.join("one.recite"), ":: one default\n-> END\n")?;
    fs::write(project.join("two.recite"), ":: two\n-> END\n")?;
    let owner = owner(claim_at(&dir.path().join("private"), None, None)?);
    assert!(owner.inbox().try_next().is_none());
    let editor_arg = std::ffi::OsString::from_vec(b"--native-\xfe".to_vec());
    let request = Request {
        project: Some(project.clone()),
        route: Some("recite://writer/write?scene=two.recite".into()),
        document: Some("two.recite".into()),
        editor: Some(EditorCommand {
            program: "editor".into(),
            args: vec![editor_arg.clone(), "--help".into()],
        }),
    };
    let socket = owner.socket.clone();
    let forwarded = thread::spawn(move || forward(&socket, &request));
    let deadline = monotonic_now() + IO_TIMEOUT;
    let incoming = loop {
        if let Some(incoming) = owner.inbox().try_next() {
            break incoming;
        }
        assert!(
            monotonic_now() < deadline,
            "forwarded options did not arrive"
        );
        thread::sleep(ACCEPT_PAUSE);
    };
    assert_eq!(
        incoming.project.as_ref().unwrap().as_os_str().as_bytes(),
        project.as_os_str().as_bytes()
    );
    assert_eq!(incoming.document, Some("two.recite".into()));
    assert_eq!(incoming.editor.unwrap().args, [editor_arg, "--help".into()]);
    incoming.reply.send(Ok(()))?;
    forwarded.join().unwrap()?;
    Ok(())
}

#[test]
fn activation_owner_releases_lease_even_with_a_duplicate_descriptor() {
    let dir = tempfile::tempdir().unwrap();
    let private = dir.path().join("private");
    let host = owner(claim_at(&private, None, None).unwrap());
    let duplicate = host._lock.try_clone().unwrap();
    drop(host);
    let replacement = owner(claim_at(&private, None, None).unwrap());
    assert!(replacement.socket.exists());
    drop(duplicate);
}

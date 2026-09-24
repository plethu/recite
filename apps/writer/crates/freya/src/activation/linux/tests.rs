use super::*;
use std::io::Write;

fn owner(activation: Activation) -> ActivationHost {
    match activation {
        Activation::Owner(owner) => owner,
        Activation::Forwarded => panic!("expected local owner"),
    }
}

#[test]
fn process_owner() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(directory) = std::env::var("RECITE_ACTIVATION_TEST_DIR") else {
        return Ok(());
    };
    let owner = owner(claim_at(Path::new(&directory), None, None)?);
    assert!(owner.inbox().try_next().is_none());
    fs::write(Path::new(&directory).join("ready"), "ready")?;
    let deadline = monotonic_now() + Duration::from_secs(8);
    while monotonic_now() < deadline {
        if let Some(request) = owner.inbox().try_next() {
            assert_eq!(request.route.as_deref(), Some("recite://writer/write"));
            request.reply.send(Ok(()))?;
            return Ok(());
        }
        thread::sleep(ACCEPT_PAUSE);
    }
    Err("child never received the forwarded route".into())
}

#[test]
fn second_process_forwards_then_lock_can_be_reclaimed() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let private = dir.path().join("private");
    private_directory(&private)?;
    let mut child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "activation::linux::tests::process_owner",
            "--nocapture",
        ])
        .env("RECITE_ACTIVATION_TEST_DIR", &private)
        .spawn()?;
    let ready = private.join("ready");
    let deadline = monotonic_now() + Duration::from_secs(8);
    while !ready.exists() && monotonic_now() < deadline {
        if let Some(status) = child.try_wait()? {
            return Err(format!("owner exited early: {status}").into());
        }
        thread::sleep(ACCEPT_PAUSE);
    }
    assert!(ready.exists(), "owner did not become ready");
    assert!(matches!(
        claim_at(&private, None, Some("recite://writer/write"))?,
        Activation::Forwarded
    ));
    assert!(child.wait()?.success());
    let replacement = owner(claim_at(&private, None, None)?);
    assert!(replacement.socket.exists());
    drop(replacement);
    assert!(!private.join("writer.sock").exists());
    assert!(private.join("writer.lock").exists());
    Ok(())
}

#[test]
fn invalid_and_oversized_frames_are_refused() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let owner = owner(claim_at(&dir.path().join("private"), None, None)?);
    let mut oversized = UnixStream::connect(&owner.socket)?;
    oversized.set_read_timeout(Some(IO_TIMEOUT))?;
    oversized.write_all(&((wire::MAX_FRAME + 1) as u32).to_be_bytes())?;
    assert!(matches!(
        read_frame::<Receipt>(&mut oversized)?,
        Receipt::Refused(_)
    ));
    let mut wrong_identity = UnixStream::connect(&owner.socket)?;
    wrong_identity.set_read_timeout(Some(IO_TIMEOUT))?;
    write_frame(
        &mut wrong_identity,
        &Request {
            project: Some(PathBuf::from("relative")),
            route: None,
        },
    )?;
    assert!(matches!(
        read_frame::<Receipt>(&mut wrong_identity)?,
        Receipt::Refused(_)
    ));
    Ok(())
}

#[test]
fn unsafe_state_entries_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o777))?;
    assert!(claim_at(dir.path(), None, None).is_err());
    fs::set_permissions(dir.path(), fs::Permissions::from_mode(0o700))?;
    let lock = dir.path().join("writer.lock");
    std::os::unix::fs::symlink("/dev/null", &lock)?;
    assert!(claim_at(dir.path(), None, None).is_err());
    Ok(())
}

#[test]
fn cold_owner_refuses_forwarding_until_the_ui_polls() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let private = dir.path().join("private");
    let owner = owner(claim_at(&private, None, None)?);
    let error = match claim_at(&private, None, Some("recite://writer/write")) {
        Err(error) => error,
        Ok(_) => return Err("cold owner accepted a request before UI startup".into()),
    };
    assert_eq!(error, "The writer is still opening a project.");
    // A refused cold-start request must never be applied later.
    assert!(owner.inbox().try_next().is_none());
    let forwarder = thread::spawn(move || claim_at(&private, None, Some("recite://writer/write")));
    let deadline = monotonic_now() + IO_TIMEOUT;
    loop {
        if let Some(request) = owner.inbox().try_next() {
            request.reply.send(Ok(()))?;
            break;
        }
        if monotonic_now() >= deadline {
            return Err("ready owner did not receive activation".into());
        }
        thread::sleep(ACCEPT_PAUSE);
    }
    assert!(matches!(
        forwarder.join().map_err(|_| "forwarder panicked")??,
        Activation::Forwarded
    ));
    Ok(())
}

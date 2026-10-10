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

#[test]
fn project_intent_must_match_the_project_encoded_in_the_route()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let make_project = |name: &str| -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = dir.path().join(name);
        fs::create_dir(&path)?;
        fs::write(
            path.join("recite.project.toml"),
            "format_version = 1\n[project]\ncontent_set = 'trial'\nversion = '1'\n",
        )?;
        Ok(path)
    };
    let first = make_project("first")?;
    let second = make_project("second")?;
    let route = format!(
        "recite://writer/write?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .append_pair("project", &first.to_string_lossy())
            .finish()
    );
    let mut request = Request {
        project: Some(first.join("recite.project.toml")),
        route: Some(route),
    };
    assert_eq!(canonical_intent(&request)?, Some(first.clone()));
    request.project = Some(second);
    assert_eq!(
        canonical_intent(&request),
        Err("The link identifies another project".into())
    );
    request.project = None;
    assert_eq!(
        canonical_intent(&request),
        Err("A project link needs a project intent".into())
    );
    request.route = None;
    request.project = Some(first.clone());
    assert_eq!(canonical_intent(&request)?, Some(first));
    request.project = Some(dir.path().join("missing"));
    assert!(
        canonical_intent(&request)
            .unwrap_err()
            .starts_with("Could not open linked project")
    );
    Ok(())
}

#[test]
fn activation_refuses_invalid_intent_and_never_queues_it() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let host = owner(claim_at(&dir.path().join("private"), None, None)?);
    assert!(host.inbox().try_next().is_none());
    for route in [
        "recite://other/write",
        "recite://writer/write?project=%2Fmissing",
    ] {
        let mut stream = UnixStream::connect(&host.socket)?;
        stream.set_read_timeout(Some(IO_TIMEOUT))?;
        write_frame(
            &mut stream,
            &Request {
                project: None,
                route: Some(route.into()),
            },
        )?;
        assert!(matches!(
            read_frame::<Receipt>(&mut stream)?,
            Receipt::Refused(_)
        ));
        assert!(host.inbox().try_next().is_none());
    }
    Ok(())
}

#[test]
fn unsafe_socket_and_lock_permissions_preserve_the_existing_entries()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let private = dir.path().join("private");
    private_directory(&private)?;
    let socket = private.join("writer.sock");
    fs::write(&socket, "unrelated content")?;
    assert!(
        matches!(claim_at(&private, None, None), Err(error) if error == "Unsafe writer socket entry")
    );
    assert_eq!(fs::read_to_string(&socket)?, "unrelated content");
    fs::remove_file(socket)?;
    let lock = private.join("writer.lock");
    fs::set_permissions(&lock, fs::Permissions::from_mode(0o644))?;
    assert!(
        matches!(claim_at(&private, None, None), Err(error) if error == "Unsafe writer lock entry")
    );
    assert!(lock.is_file());
    Ok(())
}

#[test]
fn forwarding_requires_a_queued_receipt_followed_by_an_applied_outcome()
-> Result<(), Box<dyn std::error::Error>> {
    for (receipts, expected) in [
        (vec![Receipt::Applied], "Invalid writer acknowledgement"),
        (
            vec![Receipt::Refused("Project is dirty".into())],
            "Project is dirty",
        ),
        (
            vec![Receipt::Queued, Receipt::Queued],
            "Writer did not finish opening the link",
        ),
        (
            vec![Receipt::Queued, Receipt::Refused("Link was refused".into())],
            "Link was refused",
        ),
    ] {
        let dir = tempfile::tempdir()?;
        let socket = dir.path().join("writer.sock");
        let listener = UnixListener::bind(&socket)?;
        let worker = thread::spawn(move || -> Result<(), String> {
            let (mut stream, _) = listener.accept().map_err(|error| error.to_string())?;
            let request: Request = read_frame(&mut stream)?;
            assert_eq!(request.route.as_deref(), Some("recite://writer/write"));
            for receipt in receipts {
                write_frame(&mut stream, &receipt)?;
            }
            Ok(())
        });
        assert_eq!(
            forward(
                &socket,
                &Request {
                    project: None,
                    route: Some("recite://writer/write".into())
                }
            ),
            Err(expected.into())
        );
        worker.join().map_err(|_| "receipt worker panicked")??;
    }
    Ok(())
}

#[test]
fn disconnected_activation_consumers_refuse_or_cancel_the_unapplied_request()
-> Result<(), Box<dyn std::error::Error>> {
    for disconnect_before_queue in [false, true] {
        let (mut client, mut server) = UnixStream::pair()?;
        client.set_read_timeout(Some(IO_TIMEOUT))?;
        let (sender, receiver) = mpsc::sync_channel(QUEUE_SIZE);
        let worker = thread::spawn(move || handle(&mut server, &sender, &AtomicBool::new(true)));
        if disconnect_before_queue {
            drop(receiver);
            write_frame(
                &mut client,
                &Request {
                    project: None,
                    route: None,
                },
            )?;
            assert!(matches!(
                read_frame::<Receipt>(&mut client)?,
                Receipt::Refused(message) if message == "The writer is busy handling links"
            ));
        } else {
            write_frame(
                &mut client,
                &Request {
                    project: None,
                    route: None,
                },
            )?;
            assert!(matches!(
                read_frame::<Receipt>(&mut client)?,
                Receipt::Queued
            ));
            let request = receiver.recv_timeout(IO_TIMEOUT)?;
            let cancelled = request.cancelled.clone();
            drop(request);
            assert!(matches!(
                read_frame::<Receipt>(&mut client)?,
                Receipt::Refused(message) if message.contains("request was cancelled")
            ));
            assert!(cancelled.load(Ordering::Acquire));
        }
        worker.join().map_err(|_| "activation worker panicked")??;
    }
    Ok(())
}

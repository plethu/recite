use super::*;
#[test]
fn help_avoids_project_discovery_and_editor_flag_arguments_remain_literal() {
    let parsed = LaunchOptions::parse(
        ["--project", "/definitely/missing/recite-project", "--help"].map(OsString::from),
    )
    .unwrap();
    assert_eq!(parsed.intent, Startup::Help);
    let parsed = LaunchOptions::parse(
        ["--editor", "my-editor", "--editor-arg", "--help"].map(OsString::from),
    )
    .unwrap();
    assert_eq!(
        parsed.intent,
        Startup::Writer {
            project: None,
            route: None
        }
    );
    assert_eq!(parsed.editor.unwrap().args, [OsString::from("--help")]);
}
#[test]
fn document_and_link_selection_conflicts_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n",
    )
    .unwrap();
    std::fs::write(dir.path().join("a.recite"), ":: a default\n-> END\n").unwrap();
    std::fs::write(dir.path().join("b.recite"), ":: b\n-> END\n").unwrap();
    let options = vec![
        OsString::from("--project"),
        dir.path().as_os_str().to_owned(),
        OsString::from("--document"),
        OsString::from("b.recite"),
        OsString::from("--route"),
        OsString::from("recite://writer/write?scene=a.recite"),
    ];
    assert!(
        LaunchOptions::parse(options)
            .unwrap_err()
            .contains("different documents")
    );
}
#[cfg(unix)]
#[test]
fn project_paths_and_editor_arguments_preserve_native_bytes() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    let dir = tempfile::tempdir().unwrap();
    let root = dir
        .path()
        .join(OsString::from_vec(b"project-\xff".to_vec()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("recite.project.toml"), "format_version = 1\n").unwrap();
    let arg = OsString::from_vec(b"--native-\xfe".to_vec());
    let options = LaunchOptions::parse([
        OsString::from("--project"),
        root.as_os_str().to_owned(),
        OsString::from("--editor"),
        OsString::from("editor"),
        OsString::from("--editor-arg"),
        arg.clone(),
    ])
    .unwrap();
    let Startup::Writer {
        project: Some(project),
        ..
    } = options.intent
    else {
        panic!()
    };
    assert_eq!(project.as_os_str().as_bytes(), root.as_os_str().as_bytes());
    assert_eq!(options.editor.unwrap().args, [arg]);
}

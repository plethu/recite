use recite_build::ProjectBuildRequest;
use tempfile::TempDir;

pub(super) fn project(
    with_schema: bool,
    targets: usize,
) -> Result<(TempDir, ProjectBuildRequest), Box<dyn std::error::Error>> {
    let directory = TempDir::new()?;
    std::fs::create_dir(directory.path().join("dialogue"))?;
    let mut manifest = String::from("format_version = 1\n");
    if with_schema {
        manifest.push_str("[project]\nschema = \"schema.json\"\n");
        std::fs::write(
            directory.path().join("schema.json"),
            include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
        )?;
    }
    manifest.push_str("[discovery]\nsource_roots = [\"dialogue\"]\n");
    for index in (0..targets).rev() {
        manifest.push_str(&format!("[[scenes]]\nid = \"scene.{index}\"\nasset = \"compiled/{index}.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"));
    }
    std::fs::write(directory.path().join("recite.project.toml"), manifest)?;
    let mut source = String::from(
        ":: start default speaker=hazel\n> intro@11111111111111111111\n  Bonjour, café.\n",
    );
    if with_schema {
        source.push_str("? ask@a94b83f2d8bd65101cc3 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n  What's the news?\n  -> dialogue/other.recite::other\n");
    }
    source.push_str("-> dialogue/other.recite::other\n");
    std::fs::write(directory.path().join("dialogue/start.recite"), source)?;
    std::fs::write(
        directory.path().join("dialogue/other.recite"),
        ":: other\n> end@22222222222222222222\n  Au revoir.\n-> END\n",
    )?;
    let preparation = ProjectBuildRequest::prepare(directory.path())?;
    assert!(preparation.is_ready(), "{:?}", preparation.diagnostics());
    Ok((
        directory,
        preparation.into_request().ok_or("missing request")?,
    ))
}

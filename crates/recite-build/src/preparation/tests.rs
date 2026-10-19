use super::*;

#[test]
fn prepared_sources_preserve_complete_authoring_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    std::fs::create_dir(directory.path().join("dialogue"))?;
    std::fs::write(
        directory.path().join("schema.json"),
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    )?;
    for with_schema in [false, true] {
        let schema_section = if with_schema {
            "[project]\nschema = \"schema.json\"\n"
        } else {
            ""
        };
        std::fs::write(
            directory.path().join("recite.project.toml"),
            format!(
                "format_version = 1\n{schema_section}[discovery]\nsource_roots = [\"dialogue\"]\n[[scenes]]\nid = \"start\"\nasset = \"compiled/main.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"
            ),
        )?;
        for source in [
            ":: start default\n> line@11111111111111111111\n  Café.\n-> END\n",
            ":: start default\n> line@11111111111111111111\n  Café {name}.\n-> END\n",
            ":: start default\r\n> line@11111111111111111111\r\n  Café.\r\n-> END\r\n",
            ":: start default\n:if broken(\n  malformed\n",
            ":: start default\n-> absent.recite::missing\n",
            ":: start default\n> one@11111111111111111111\n  One.\n> two@11111111111111111111\n  Two.\n-> END\n",
            ":: start default\n! immediate entered_start()\n-> END\n",
        ] {
            std::fs::write(directory.path().join("dialogue/start.recite"), source)?;
            let discovery = discover_project(directory.path())?;
            let schema = discovery
                .manifest()
                .load_schema()?
                .and_then(|loaded| loaded.into_report().schema);
            let mut expected = validate_sources(discovery.documents(), schema.as_ref(), true)?;
            sort_diagnostics(&mut expected);
            let actual = ProjectBuildRequest::prepare(directory.path())?;
            assert_eq!(
                actual.diagnostics(),
                expected,
                "schema={with_schema}: {source}"
            );
        }
    }
    Ok(())
}

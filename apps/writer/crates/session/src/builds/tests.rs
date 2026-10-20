use super::*;
use std::fs;

#[test]
fn target_listing_is_stable_by_asset_and_preserves_source_order_within_each_output()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[[scenes]]\nid='first'\nasset='z.recitec'\nblock='start'\n[[scenes]]\nid='second'\nasset='a.recitec'\nblock='start'\n[[scenes]]\nid='third'\nasset='z.recitec'\nblock='start'\n",
    )?;
    fs::write(
        directory.path().join("scene.recite"),
        ":: start default\n-> END\n",
    )?;
    assert_eq!(
        targets(directory.path())?,
        vec![
            ("a.recitec".into(), vec!["second".into()]),
            ("z.recitec".into(), vec!["first".into(), "third".into()])
        ]
    );
    Ok(())
}

#[test]
fn listing_rejects_ancestor_discovery_after_the_owned_manifest_disappears()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n",
    )?;
    let child = directory.path().join("child");
    fs::create_dir(&child)?;
    let manifest = child.join("recite.project.toml");
    fs::write(&manifest, "format_version=1\n")?;
    assert!(targets(&child)?.is_empty());
    fs::remove_file(manifest)?;
    assert!(
        targets(&child)
            .unwrap_err()
            .contains("project root changed")
    );
    Ok(())
}

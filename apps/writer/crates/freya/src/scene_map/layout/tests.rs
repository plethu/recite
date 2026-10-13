use super::*;

#[test]
fn cycles_and_convergence_keep_one_position_per_beat() -> Result<(), Box<dyn std::error::Error>> {
    for example in &recite_writer_model::WRITER_EXAMPLES[..2] {
        let workbench = example.open()?;
        let blocks = workbench.document().script()?;
        let links = recite_writer_model::scene_links(&blocks);
        let nodes = layout(&blocks, &links, WIDTH);
        assert_eq!(nodes.len(), blocks.len());
        for (index, node) in nodes.iter().enumerate() {
            assert!(node.width > 40.);
            assert!(node.x + node.width < WIDTH);
            assert!(
                !nodes[..index]
                    .iter()
                    .any(|other| other.x == node.x && other.y == node.y)
            );
        }
    }
    Ok(())
}

#[test]
#[ignore = "explicit geometry reconstruction timing workload"]
fn map_geometry_reconstruction_timing() -> Result<(), Box<dyn std::error::Error>> {
    use super::super::topology::Topology;
    use criterion::measurement::{Measurement, WallTime};
    let mut samples = Vec::new();
    let authored = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let mut cases = vec![(
        "authored_hub".to_owned(),
        authored.document().script_snapshot()?,
    )];
    for beats in [100, 1000, 10_000] {
        let document = recite_writer_model::Document::new(recite_writer_model::workload::source(
            beats * 5,
            0,
            5,
        ))?;
        cases.push((format!("linear_{beats}"), document.script_snapshot()?));
    }
    let mut source = String::from(":: hub default\n");
    for index in 0..1000 {
        source.push_str(&format!(
            "? reply_{index}@{index:020x}\n  Choose {index}.\n  -> beat_{index}\n"
        ));
    }
    for index in 0..1000 {
        source.push_str(&format!(":: beat_{index}\n-> hub\n"));
    }
    let fanout = recite_writer_model::Document::new(&source)?;
    assert!(fanout.diagnostics().is_empty());
    cases.push(("fanout_1000".into(), fanout.script_snapshot()?));
    for (name, blocks) in cases {
        for _ in 0..20 {
            let start = WallTime.start();
            let graph = Topology::new(blocks.clone(), &Default::default());
            let milliseconds = WallTime.end(start).as_secs_f64() * 1000.;
            assert_eq!(graph.nodes.len(), blocks.len());
            samples.push(serde_json::json!({"shape":name,"beats":blocks.len(),
                "links":graph.links.len(),"milliseconds":milliseconds}));
            std::hint::black_box(graph);
        }
    }
    println!(
        "{}",
        serde_json::json!({"operation":"topology_layout_routing","samples":samples,
        "profile":if cfg!(debug_assertions) {"test"} else {"release"},"native_frame_pacing":false})
    );
    Ok(())
}

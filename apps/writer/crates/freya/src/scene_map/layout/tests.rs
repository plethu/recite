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

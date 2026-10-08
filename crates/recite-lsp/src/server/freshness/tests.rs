use super::Epochs;
use proptest::prelude::*;

const PARTITIONS: [&str; 3] = ["left", "right", "third"];

proptest! {
    #[test]
    fn fences_follow_topology_and_partition_history(
        actions in prop::collection::vec(prop::option::of(0usize..3), 1..32),
    ) {
        let mut epochs = Epochs::default();
        let mut topology = 0;
        let mut revision = 0;
        let mut partitions = [0; 3];
        let mut tickets = Vec::new();
        for action in actions {
            for scope in [None, Some(0), Some(1), Some(2)] {
                let generation = scope.map_or(revision, |index| partitions[index]);
                let fence = epochs.fence(scope.map(|index| PARTITIONS[index]));
                tickets.push((scope, topology, generation, fence));
            }
            let before = epochs.clone();
            prop_assert_eq!(epochs.advance(action.map(|index| PARTITIONS[index])), Some(()));
            revision += 1;
            if let Some(index) = action {
                partitions[index] += 1;
            } else {
                topology += 1;
            }
            prop_assert_eq!(epochs.same_topology(&before), action.is_some());
            for (scope, issued_topology, issued_generation, fence) in &tickets {
                let generation = scope.map_or(revision, |index| partitions[index]);
                let expected = *issued_topology == topology && *issued_generation == generation;
                prop_assert_eq!(epochs.matches(fence), expected);
            }
        }
    }
}

#[test]
fn generation_exhaustion_fails_without_wrapping() {
    let mut global = Epochs {
        revision: u64::MAX,
        ..Epochs::default()
    };
    assert_eq!(global.advance(Some("left")), None);
    assert_eq!(global.revision, u64::MAX);
    assert!(global.partitions.is_empty());

    let mut topology = Epochs {
        topology: u64::MAX,
        ..Epochs::default()
    };
    assert_eq!(topology.advance(None), None);
    assert_eq!(topology.topology, u64::MAX);

    let mut partition = Epochs::default();
    partition.partitions.insert("left".into(), u64::MAX);
    assert_eq!(partition.advance(Some("left")), None);
    assert_eq!(partition.partitions["left"], u64::MAX);
}

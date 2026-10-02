use std::cell::Cell;

use recite_compiler::authoring::{
    AuthoringEditError, AuthoringKernel, AuthoringRequest, Interrupted, SavedDocument, WorkControl,
};
use recite_core::{DocumentKey, SourcePosition};

struct Budget(Cell<usize>);
impl WorkControl for Budget {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        let remaining = self.0.get().checked_sub(1).ok_or(Interrupted)?;
        self.0.set(remaining);
        Ok(())
    }
}

#[test]
fn large_plan_validation_stops_between_edits_and_retains_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let key = DocumentKey::new("large.recite")?;
    let source = format!(":: start default\r\n{}", "-> start\r\n".repeat(1000));
    let mut kernel = AuthoringKernel::new();
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(key.clone(), source)],
        [],
    ))?;
    let snapshot = kernel.snapshot();
    let plan = snapshot.plan_rename_block(&key, SourcePosition::new(1, 4)?, "renamed")?;
    assert_eq!(plan.edits().len(), 1001);
    for allowance in [0, 2, 100, 500, 1000] {
        let budget = Budget(Cell::new(allowance));
        assert!(matches!(
            plan.validate_with_control(&snapshot.query(&budget)),
            Err(AuthoringEditError::Interrupted(_))
        ));
    }
    plan.validate(snapshot)?;
    assert_eq!(
        snapshot.plan_rename_block(&key, SourcePosition::new(1, 4)?, "renamed")?,
        plan
    );
    Ok(())
}

use super::*;
impl Job {
    pub(super) fn not_started(self, error: String) -> Completed {
        let result = match self.work {
            Work::Start(work) => Ok(Value::Started(work.not_started(error))),
            Work::Publish(work) => Ok(Value::Published {
                receipt: Box::new(work.not_started(error)),
                rows: vec![],
            }),
            Work::Refresh(_) => Err(error),
            Work::Declaration(work) => Ok(Value::Declaration(Box::new(
                declaration::Prepared::new(work.not_started(error)),
            ))),
        };
        Completed {
            request: self.request,
            result,
        }
    }
    pub(super) fn execute(self) -> Completed {
        let result = match self.work {
            Work::Start(work) => Ok(Value::Started(work.execute())),
            Work::Publish(work) => {
                let receipt = Box::new(work.execute());
                let rows = receipt
                    .schema()
                    .map(|schema| entries::entries(schema).into_iter().map(Row::new).collect())
                    .unwrap_or_default();
                Ok(Value::Published { receipt, rows })
            }
            Work::Declaration(work) => Ok(Value::Declaration(Box::new(
                declaration::Prepared::new(work.execute()),
            ))),
            Work::Refresh(work) => std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                work.prepare(&recite_compiler::authoring::CancellationToken::new())
                    .map(|prepared| Value::Refreshed(Box::new(prepared)))
                    .map_err(|error| error.to_string())
            }))
            .unwrap_or_else(|_| Err("Project analysis stopped unexpectedly.".into())),
        };
        Completed {
            request: self.request,
            result,
        }
    }
}

//! معاملات كتابة متعددة الخطوات — ذرّية SQLite فقط (بدون I/O خارجي).
//!
//! Fault injection للاختبارات:
//! `test_set_tx_fail_stage(Some("after_first"))` أو `Some("before_commit")`.

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    static TX_FAIL_STAGE: Cell<Option<&'static str>> = const { Cell::new(None) };
}

#[cfg(test)]
pub fn test_set_tx_fail_stage(stage: Option<&'static str>) {
    TX_FAIL_STAGE.with(|c| c.set(stage));
}

pub fn checkpoint_after_first() -> Result<(), String> {
    maybe_fail("after_first")
}

pub fn checkpoint_before_commit() -> Result<(), String> {
    maybe_fail("before_commit")
}

fn maybe_fail(stage: &'static str) -> Result<(), String> {
    #[cfg(test)]
    {
        return TX_FAIL_STAGE.with(|c| {
            if c.get() == Some(stage) {
                Err(format!("test injected fail at {stage}"))
            } else {
                Ok(())
            }
        });
    }
    #[cfg(not(test))]
    {
        let _ = stage;
        Ok(())
    }
}

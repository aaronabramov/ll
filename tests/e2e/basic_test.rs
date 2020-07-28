use crate::utils::test_drain::TestDrain;
use anyhow::Result;
use k9::*;
use ll::Logger;

#[test]
fn basic_events_test() -> Result<()> {
    let test_drain = TestDrain::new();
    let l = Logger::new();
    l.add_drain(Box::new(test_drain.clone()));

    l.with_event("test", |_e| {
        let _r = 1 + 1;
        Ok(())
    })?;

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "[<REDACTED>] test                                                        |     0ms"
    );

    Ok(())
}

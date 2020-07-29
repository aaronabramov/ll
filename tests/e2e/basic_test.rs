use crate::utils::test_drain::TestDrain;
use anyhow::Result;
use k9::*;
use ll::logger::Logger;

fn setup() -> (Logger, TestDrain) {
    let test_drain = TestDrain::new();
    let l = Logger::new();
    l.add_drain(Box::new(test_drain.clone()));
    (l, test_drain)
}

#[test]
fn basic_events_test() -> Result<()> {
    let (l, test_drain) = setup();

    l.with_event("test", |_e| {
        let _r = 1 + 1;
        Ok(())
    })?;

    l.with_event("test_with_data", |e| {
        e.add_data("hello", "hi");
        e.add_data("int", 5);
        e.add_data("float", 5.98);
        Ok(())
    })?;

    l.with_event("test_3", |_e| Ok(()))?;

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "
[<REDACTED>] test                                                        |     0ms
[<REDACTED>] test_with_data                                              |     0ms
  |      float: 5.98
  |      hello: hi
  |      int: 5
[<REDACTED>] test_3                                                      |     0ms

"
    );

    Ok(())
}

#[test]
fn error_chain_test() -> Result<()> {
    let (l, test_drain) = setup();

    let result = l.with_event("top_level", |e| {
        e.add_data("top_level_data", 5);

        l.with_event("1_level", |e2| {
            e2.add_data("1_level_data", 9);
            l.with_event("2_level", |_| {
                anyhow::ensure!(false, "oh noes, this fails");
                Ok(())
            })
        })?;
        Ok(())
    });

    assert_matches_inline_snapshot!(
        format!("\n{:?}\n", result.unwrap_err()),
        "
[inside event] top_level
    top_level_data: 5


Caused by:
    0: [inside event] 1_level
           1_level_data: 9
       
    1: [inside event] 2_level
    2: oh noes, this fails
"
    );

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "
[<REDACTED>] [ERR] 2_level                                               |     0ms
[<REDACTED>] [ERR] 1_level                                               |     0ms
  |      1_level_data: 9
[<REDACTED>] [ERR] top_level                                             |     0ms
  |      top_level_data: 5

"
    );
    Ok(())
}

use crate::utils::test_drain::TestDrain;
use anyhow::Result;
use k9::*;
use ll::level::Level;
use ll::logger::Logger;
use std::sync::Arc;

fn setup() -> (Logger, TestDrain) {
    let test_drain = TestDrain::new();
    let mut l = Logger::new();
    l.add_drain(Arc::new(test_drain.clone()));
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

#[test]
fn logger_data_test() -> Result<()> {
    let (mut l, test_drain) = setup();

    l.add_data("process_id", 123);

    l.with_event("has_process_id", |_| Ok(()))?;

    #[allow(clippy::redundant_clone)]
    let mut l2 = l.clone();
    l2.add_data("request_id", 234);
    l2.with_event("has_process_and_request_id", |_| Ok(()))?;

    #[allow(clippy::redundant_clone)]
    let mut l3 = l2.clone();
    l3.add_data("request_id #dontprint", 592);
    l3.with_event("wont_print_request_id", |_| Ok(()))?;

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "
[<REDACTED>] has_process_id                                              |     0ms
  |      process_id: 123
[<REDACTED>] has_process_and_request_id                                  |     0ms
  |      process_id: 123
  |      request_id: 234
[<REDACTED>] wont_print_request_id                                       |     0ms
  |      process_id: 123

"
    );
    Ok(())
}

#[test]
fn log_levels_test() -> Result<()> {
    let (mut l, test_drain) = setup();

    l.set_log_level(Level::Trace);
    l.with_event("level_set_to_trace", |_| Ok(()))?;
    l.with_event("trace #trace", |_| Ok(()))?;
    l.with_event("debug #debug", |_| Ok(()))?;
    l.with_event("info #info", |_| Ok(()))?;
    l.with_event("default #info", |_| Ok(()))?;
    l.with_event("log_datadata", |e| {
        e.add_data("data_trace #trace", true);
        e.add_data("data_debug #debug", true);
        e.add_data("data_info #info", true);
        e.add_data("data_default", true);
        Ok(())
    })?;

    l.set_log_level(Level::Debug);
    l.with_event("level_set_to_debug", |_| Ok(()))?;
    l.with_event("trace #trace", |_| Ok(()))?;
    l.with_event("debug #debug", |_| Ok(()))?;
    l.with_event("info #info", |_| Ok(()))?;
    l.with_event("default #info", |_| Ok(()))?;
    l.with_event("log_datadata", |e| {
        e.add_data("data_trace #trace", true);
        e.add_data("data_debug #debug", true);
        e.add_data("data_info #info", true);
        e.add_data("data_default", true);
        Ok(())
    })?;

    l.set_log_level(Level::Info);
    l.with_event("level_set_to_info", |_| Ok(()))?;
    l.with_event("trace #trace", |_| Ok(()))?;
    l.with_event("debug #debug", |_| Ok(()))?;
    l.with_event("info #info", |_| Ok(()))?;
    l.with_event("default #info", |_| Ok(()))?;
    l.with_event("log_datadata", |e| {
        e.add_data("data_trace #trace", true);
        e.add_data("data_debug #debug", true);
        e.add_data("data_info #info", true);
        e.add_data("data_default", true);
        Ok(())
    })?;

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "
[<REDACTED>] level_set_to_trace                                          |     0ms
[<REDACTED>] trace                                                       |     0ms
[<REDACTED>] debug                                                       |     0ms
[<REDACTED>] info                                                        |     0ms
[<REDACTED>] default                                                     |     0ms
[<REDACTED>] log_datadata                                                |     0ms
  |      data_debug: true
  |      data_default: true
  |      data_info: true
  |      data_trace: true
[<REDACTED>] level_set_to_debug                                          |     0ms
[<REDACTED>] debug                                                       |     0ms
[<REDACTED>] info                                                        |     0ms
[<REDACTED>] default                                                     |     0ms
[<REDACTED>] log_datadata                                                |     0ms
  |      data_debug: true
  |      data_default: true
  |      data_info: true
[<REDACTED>] level_set_to_info                                           |     0ms
[<REDACTED>] info                                                        |     0ms
[<REDACTED>] default                                                     |     0ms
[<REDACTED>] log_datadata                                                |     0ms
  |      data_default: true
  |      data_info: true

"
    );
    Ok(())
}

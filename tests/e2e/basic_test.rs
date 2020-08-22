use anyhow::Result;
use k9::*;
use ll::drains::stdout::StringDrain;
use ll::{Level, Logger};
use std::sync::{Arc, Mutex};

fn setup() -> (Logger, StringDrain) {
    let string_drain = StringDrain::new();
    let mut l = Logger::new();
    l.add_drain(Arc::new(string_drain.clone()));
    (l, string_drain)
}

#[test]
fn basic_events_test() -> Result<()> {
    let (l, test_drain) = setup();

    l.event("test", |_e| {
        let _r = 1 + 1;
        Ok(())
    })?;

    l.event("test_with_data", |e| {
        e.add_data("hello", "hi");
        e.add_data("int", 5);
        e.add_data("float", 5.98);
        e.set_error_msg("this is a custom error message that will be attached to Event");
        Ok(())
    })?;

    l.event("test_3", |_e| Ok(()))?;

    l.event("will_be_discarded", |e| {
        e.discard();
        Ok(())
    })
    .unwrap();

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "
[ ] test                                                        
[ ] [ERR] test_with_data                                        
  |      float: 5.98
  |      hello: hi
  |      int: 5
  |
  |  this is a custom error message that will be attached to Event
[ ] test_3                                                      

"
    );

    Ok(())
}

#[test]
fn error_chain_test() -> Result<()> {
    let (l, test_drain) = setup();

    let result = l.event("top_level", |e| {
        e.add_data("top_level_data", 5);

        l.event("1_level", |e2| {
            e2.add_data("1_level_data", 9);
            l.event("2_level", |_| {
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
[ ] [ERR] 2_level                                               
[ ] [ERR] 1_level                                               
  |      1_level_data: 9
[ ] [ERR] top_level                                             
  |      top_level_data: 5

"
    );
    Ok(())
}

#[test]
fn logger_data_test() -> Result<()> {
    let (mut l, test_drain) = setup();

    l.add_data("process_id", 123);

    l.event("has_process_id", |_| Ok(()))?;

    #[allow(clippy::redundant_clone)]
    let mut l2 = l.clone();
    l2.add_data("request_id", 234);
    l2.event("has_process_and_request_id", |_| Ok(()))?;

    #[allow(clippy::redundant_clone)]
    let mut l3 = l2.clone();
    l3.add_data("request_id #dontprint", 592);
    l3.event("wont_print_request_id", |_| Ok(()))?;

    #[allow(clippy::redundant_clone)]
    let mut l4 = l3.clone();
    l4.set_event_name_prefix("my_service");
    l4.event("wont_print_request_id", |_| Ok(()))?;

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "
[ ] has_process_id                                              
  |      process_id: 123
[ ] has_process_and_request_id                                  
  |      process_id: 123
  |      request_id: 234
[ ] wont_print_request_id                                       
  |      process_id: 123
[ ] my_service:wont_print_request_id                            
  |      process_id: 123

"
    );
    Ok(())
}

#[test]
fn log_levels_test() -> Result<()> {
    let (mut l, test_drain) = setup();

    l.set_log_level(Level::Trace);
    l.event("level_set_to_trace", |_| Ok(()))?;
    l.event("trace #trace", |_| Ok(()))?;
    l.event("debug #debug", |_| Ok(()))?;
    l.event("info #info", |_| Ok(()))?;
    l.event("default #info", |_| Ok(()))?;
    l.event("log_datadata", |e| {
        e.add_data("data_trace #trace", true);
        e.add_data("data_debug #debug", true);
        e.add_data("data_info #info", true);
        e.add_data("data_default", true);
        Ok(())
    })?;

    l.set_log_level(Level::Debug);
    l.event("level_set_to_debug", |_| Ok(()))?;
    l.event("trace #trace", |_| Ok(()))?;
    l.event("debug #debug", |_| Ok(()))?;
    l.event("info #info", |_| Ok(()))?;
    l.event("default #info", |_| Ok(()))?;
    l.event("log_datadata", |e| {
        e.add_data("data_trace #trace", true);
        e.add_data("data_debug #debug", true);
        e.add_data("data_info #info", true);
        e.add_data("data_default", true);
        Ok(())
    })?;

    l.set_log_level(Level::Info);
    l.event("level_set_to_info", |_| Ok(()))?;
    l.event("trace #trace", |_| Ok(()))?;
    l.event("debug #debug", |_| Ok(()))?;
    l.event("info #info", |_| Ok(()))?;
    l.event("default #info", |_| Ok(()))?;
    l.event("log_datadata", |e| {
        e.add_data("data_trace #trace", true);
        e.add_data("data_debug #debug", true);
        e.add_data("data_info #info", true);
        e.add_data("data_default", true);
        Ok(())
    })?;

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "
[ ] level_set_to_trace                                          
[ ] trace                                                       
[ ] debug                                                       
[ ] info                                                        
[ ] default                                                     
[ ] log_datadata                                                
  |      data_debug: true
  |      data_default: true
  |      data_info: true
  |      data_trace: true
[ ] level_set_to_debug                                          
[ ] debug                                                       
[ ] info                                                        
[ ] default                                                     
[ ] log_datadata                                                
  |      data_debug: true
  |      data_default: true
  |      data_info: true
[ ] level_set_to_info                                           
[ ] info                                                        
[ ] default                                                     
[ ] log_datadata                                                
  |      data_default: true
  |      data_info: true

"
    );
    Ok(())
}

#[test]
fn async_test() -> Result<()> {
    let mut rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let (l, test_drain) = setup();

        l.async_event("async_event", |e| async move {
            e.add_data("async_data", 5);
            let block = async {};
            block.await;
            Ok(())
        })
        .await?;

        assert_matches_inline_snapshot!(
            test_drain.to_string(),
            "
[ ] async_event                                                 
  |      async_data: 5

"
        );
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(())
}

#[test]
fn custom_drain_test() {
    let s = Arc::new(Mutex::new(String::new()));
    struct AnalyticsDBDrain(Arc<Mutex<String>>);

    impl ll::Drain for AnalyticsDBDrain {
        fn log_event(&self, e: &ll::Event) {
            let mut s = self.0.lock().unwrap();
            s.push_str(&e.name);
            s.push_str(" ");
            for (k, entry) in &e.data.map {
                let v = &entry.0;
                let tags = &entry.1;
                s.push_str(&format!("{:?}", tags));
                match v {
                    ll::DataValue::Int(i) => s.push_str(&format!("{}: int: {}", k, i)),
                    _ => s.push_str(&format!("{}: {:?}", k, v)),
                }
            }
        }
    }

    let mut l = ll::Logger::stdout();
    let drain = Arc::new(AnalyticsDBDrain(s.clone()));
    l.add_drain(drain);

    l.event("some_event #some_tag", |_| Ok(())).unwrap();

    l.event("other_event", |e| {
        e.add_data("data #dontprint", 1);
        Ok(())
    })
    .unwrap();

    assert_matches_inline_snapshot!(
        s.lock().unwrap().clone(),
        "some_event other_event {\"dontprint\"}data: int: 1"
    );
}

#[test]
fn nested_loggers_test() -> Result<()> {
    let (mut l, test_drain) = setup();

    l.add_data("process_id", 123);
    l.event("has_process_id", |_| Ok(()))?;

    let l2 = l.nest("my_app");
    l2.event("some_app_event", |_| Ok(()))?;

    let mut l3 = l2.nest("db");
    l3.add_data("db_connection_id", 234);
    l3.event("some_db_event", |_| Ok(()))?;

    l2.event("another_app_event", |_| Ok(()))?;

    assert_matches_inline_snapshot!(
        test_drain.to_string(),
        "
[ ] has_process_id                                              
  |      process_id: 123
[ ] my_app:some_app_event                                       
  |      process_id: 123
[ ] my_app:db:some_db_event                                     
  |      db_connection_id: 234
  |      process_id: 123
[ ] my_app:another_app_event                                    
  |      process_id: 123

"
    );
    Ok(())
}

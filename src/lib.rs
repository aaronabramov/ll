#![allow(clippy::new_without_default)]

/*!
# ll - Lightweight logging library with support for async/await

The main focus of this library is to provide a lightweight API that can
wrap parts of the execution flow, measure how long it took to run these flows
and log results as separate events to different event drains (log handlers).


```
# async fn db_query() -> Vec<i32> {
#   vec![1, 2, 3]
# }
# #[tokio::main]
# async fn main() -> () {
let l = ll::Logger::stdout(); // new logger with STDOUT drain configured

l.event("expensive_computation", |e| {
    let x = 10000;
    e.add_data("elements_in_vec", x);
    Ok((1..x).into_iter().collect::<Vec<i32>>())
}).unwrap();

l.async_event("db_query", |e| async move {
    let result = db_query().await;
    e.add_data("rows_returned", result.len());
    Ok(result)
}).await.unwrap();

# }

```

Will result in the following output being printed to STDERR:
```txt
 [2020-07-29T23:46:48Z] expensive_computation                                       |     10ms
 |      elements_in_vec: 10000
 [2020-07-29T23:46:48Z] db_query                                                    |     55ms
   |      rows_returned: 3
```


# Adding default data to a logger

All loggers are clonable objects that can be configured and passed down to different parts of the app.

```
# fn get_request_id() -> &'static str {
#     "ab49f92h49"
# }
let mut l = ll::Logger::stdout();

// add hostname that will be present in all events
l.add_data("hostname", "devserver.123.com");

fn handle_http_request(mut request_logger: ll::Logger) {
    // Add a `request id` that will be logged only for events within
    // specific request
    request_logger.add_data("request_id", get_request_id());

    request_logger
        .event("http_request", |_| {
            // do things...
            Ok(())
        })
        .unwrap();
}

// Clone the logger and pass it to every incoming http request
handle_http_request(l.clone());
```

will result in:
```txt
[2020-07-30T00:02:48Z] http_request                                                |     0ms
  |      hostname: devserver.123.com
  |      request_id: ab49f92h49
```


# Setting the log level

ll comes with three levels of logging
- Info (Will log only Info level events, which is default)
- Debug (Will log Debug and Info)
- Trace (Will log everything)

By default ll uses `Info` level and will only log `Info` events and data, but it can be configured
```
# let mut l = ll::Logger::new();
l.set_log_level(ll::Level::Trace);
```

Events and data properties can use hashtags to configure their individual log levels.
```
let mut l = ll::Logger::stdout();
l.set_log_level(ll::Level::Debug);

l.event("wont_be_logged #trace", |_| Ok(())).unwrap();

l.event("will_be_logged #debug", |e| {
    e.add_data("this_data_will_be_logged #info", true);
    e.add_data("this_data_wont #trace", true);
    Ok(())
})
.unwrap();
```

```txt
[2020-07-30T00:10:34Z] will_be_logged                                              |     0ms
  |      this_data_will_be_logged: true

```

# Custom drains

Most of the time events need to be not only logged to stdout, but also different log processing platforms
or even a `.log` file in the filesystem.

ll logger can proxy every event to multiple drains. Implementing a drain is easy and only requires implementing a single `log_event` function

```
struct DbgDrain;

impl ll::Drain for DbgDrain {
    fn log_event(&self, e: &ll::Event) {
        dbg!(&e.name);
    }
}

// new() returns a logger instance with no drains setup
let mut l = ll::Logger::new();

l.add_drain(std::sync::Arc::new(DbgDrain));

l.event("hello", |_| Ok(())).unwrap();
```

```txt
[main.rs:251] &e.name = "hello"
```

# Hashtags

Every event can have any number of hashtags in its name (space separated strings that are prefixed with `#`)
These hastags are parsed into a `BTreeSet<String>` when event is created and can later affect the way we handle event logging in drains.

For example, if there's a need to log something to a database but not to STDOUT, `#dontprint` hashtag can be used. `StdoutDrain` will later
check for the existence of this hashtag, and if present, it will skip printing it out to STDOUT alltogether.

```
# struct AnalyticsDBDrain;
# impl ll::Drain for AnalyticsDBDrain {
#    fn log_event(&self, _e: &ll::Event) {}
# }
let mut l = ll::Logger::stdout();
l.add_drain(std::sync::Arc::new(AnalyticsDBDrain));

l.event(
    "will_be_logged_to_analytics_db_but_not_printed #dontprint",
    |_| Ok(()),
)
.unwrap();

l.event("will_be_printed", |e| {
    e.add_data("but_this_data_wont #dontprint", 1);
    Ok(())
})
.unwrap();
```

```txt
[2020-07-30T00:21:47Z] will_be_printed                                             |     0ms
```
*/
pub mod drains;
mod event_data;
mod events;
mod level;
mod logger;
mod types;

mod utils;

pub use drains::Drain;
pub use event_data::DataValue;
pub use events::{Event, OngoingEvent};
pub use level::Level;
pub use logger::Logger;

pub use logger::{async_event, event};

pub mod ll2;

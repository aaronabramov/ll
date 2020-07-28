use ll::drains::stdout::{make_string, TimestampFormat, DONTPRINT_TAG};
use ll::drains::Drain;
use ll::Event;
use std::sync::{Arc, Mutex};

pub fn strip_ansi(s: &str) -> String {
    String::from_utf8(
        strip_ansi_escapes::strip(s).expect("Cant strip ANSI escape characters from a string"),
    )
    .expect("not a utf8 string")
}

#[derive(Clone)]
pub struct TestDrain {
    pub output: Arc<Mutex<String>>,
    pub strip_ansi: bool,
}

impl TestDrain {
    pub fn new() -> Self {
        Self {
            output: Arc::new(Mutex::new(String::new())),
            strip_ansi: true,
        }
    }
}

impl Drain for TestDrain {
    fn log_event(&self, event: &Event) {
        if event.tags.contains(DONTPRINT_TAG) {
            return;
        }
        let mut result = make_string(event, TimestampFormat::Redacted);
        if self.strip_ansi {
            result = strip_ansi(&result);
        }
        self.output.lock().expect("poisoned lock").push_str(&result);
    }
}

impl std::fmt::Display for TestDrain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.output.lock().expect("poisoned lock");
        write!(f, "{}", &s)
    }
}

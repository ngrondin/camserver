use std::time::{SystemTime, UNIX_EPOCH};

fn ts() -> u32 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_millis()
}

pub struct Timer {
    start: u32
}

#[allow(dead_code)]
impl Timer {
    pub fn new() -> Self {
        Timer { start: ts() }
    }

    pub fn reset(&mut self) {
        self.start = ts();
    }

    pub fn mark(&self) -> u32 {
        let now = ts();
        let dur = if now > self.start { now - self.start } else {0};
        dur
    }
}
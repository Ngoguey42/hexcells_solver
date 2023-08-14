use std::fmt;
use std::time::{Duration, Instant};
use Error;

#[derive(Debug)]
pub struct Timeout;

impl Error for Timeout {}

impl fmt::Display for Timeout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Timeout")
    }
}

/// Represents the runtime environment for the solver, responsible for managing timeouts.
pub struct Env {
    start_time: Instant,
    max_duration: Duration,
}

impl Env {
    pub fn new(max_duration: u64) -> Env {
        let start_time = Instant::now();
        let max_duration = Duration::from_secs(max_duration);
        Env {
            start_time,
            max_duration,
        }
    }

    pub fn reset_timer(&mut self) {
        self.start_time = Instant::now();
    }

    pub fn check_timeout(&self) -> Result<(), Box<dyn Error>> {
        if self.start_time.elapsed() >= self.max_duration {
            Err(Box::new(Timeout))
        } else {
            Ok(())
        }
    }
}

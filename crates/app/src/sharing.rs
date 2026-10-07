//! Phone sharing follows its setting. A start that fails (no home network, no free port) is
//! tried again at most every 30 s, or at once when the user switches sharing on or the pairing
//! code changes, and says why only once.

use std::time::{Duration, Instant};

/// How long a failed start waits before it is tried again.
pub const RETRY: Duration = Duration::from_secs(30);

/// What runs (`P`: the server and what it shares) and the last start that failed.
pub struct PhoneSharing<P> {
    pub running: Option<P>,
    /// The pairing code that start was for, and when.
    failed: Option<(String, Instant)>,
}

impl<P> Default for PhoneSharing<P> {
    fn default() -> Self {
        PhoneSharing { running: None, failed: None }
    }
}

impl<P> PhoneSharing<P> {
    /// Follows the setting (`on`) and the pairing code. `serves`: the running server still shares
    /// this code. `start` starts one, or says why it can't. Returns the notice to show, if any.
    pub fn sync(&mut self, on: bool, token: &str, now: Instant, serves: impl Fn(&P, &str) -> bool, start: impl FnOnce() -> Result<P, String>) -> Option<String> {
        if !on {
            // Switching it on again tries at once.
            self.running = None;
            self.failed = None;
            return None;
        }
        if self.running.as_ref().is_some_and(|p| serves(p, token)) {
            return None;
        }
        self.running = None;
        // The same code failed before: wait out the retry, and don't repeat why.
        let again = match &self.failed {
            Some((code, at)) if code == token => {
                if now.saturating_duration_since(*at) < RETRY {
                    return None;
                }
                true
            }
            _ => false,
        };
        match start() {
            Ok(p) => {
                self.running = Some(p);
                self.failed = None;
                None
            }
            Err(why) => {
                self.failed = Some((token.to_string(), now));
                (!again).then_some(why)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_start_that_fails_is_tried_again_every_30_seconds_and_said_once() {
        let mut sharing: PhoneSharing<String> = PhoneSharing::default();
        let t0 = Instant::now();
        let tries = std::cell::Cell::new(0);
        // `works`: whether a start succeeds (no home network otherwise).
        let mut sync = |on: bool, token: &str, secs: f64, works: bool| {
            let start = || {
                tries.set(tries.get() + 1);
                if works { Ok(format!("http://192.168.1.2:47330/m?t={token}")) } else { Err("Phone sharing: no home network found".to_string()) }
            };
            sharing.sync(on, token, t0 + Duration::from_secs_f64(secs), |link, token| link.ends_with(token), start)
        };
        assert_eq!(sync(true, "code", 0.0, false).as_deref(), Some("Phone sharing: no home network found"));
        // Every message runs this: none of them try again (or repeat the notice) for 30 s.
        for n in 1..120 {
            assert_eq!(sync(true, "code", n as f64 * 0.25, false), None);
        }
        assert_eq!(tries.get(), 1);
        assert_eq!(sync(true, "code", 30.0, false), None, "tried again, quietly");
        assert_eq!(sync(true, "code", 31.0, false), None);
        assert_eq!(tries.get(), 2);
        // Switched off and on again: tried at once, and the reason shown again.
        assert_eq!(sync(false, "code", 40.0, false), None);
        assert!(sync(true, "code", 41.0, false).is_some());
        assert_eq!(tries.get(), 3);
        // A new pairing code: at once too.
        assert!(sync(true, "new", 42.0, false).is_some());
        assert_eq!(tries.get(), 4);
        // The network is back: running, and not started again while it serves this code.
        assert_eq!(sync(true, "new", 72.0, true), None);
        assert_eq!(sync(true, "new", 73.0, true), None);
        assert_eq!(tries.get(), 5);
        assert!(sharing.running.is_some());
    }
}

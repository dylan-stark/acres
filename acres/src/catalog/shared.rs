//! A token bucket kept in a file, so separate processes share one rate limit (ADR 0016).
//!
//! Each `acres-cli` run is its own process, and the TUI can run beside a script. An in-memory
//! bucket would give each of them a full burst, so together they could pass the API's 60 a
//! minute. Instead, the bucket's state (tokens left and when it was last refilled) lives in one
//! file in the cache directory. Taking a token locks the file, refills, takes or reports the
//! wait, writes the state back and unlocks. The lock is held for a read and a write, never across
//! a request.
//!
//! The file holds one line, `<tokens> <refilled>`, where `refilled` is seconds since the Unix
//! epoch. Wall-clock time is used because it is the only clock processes share. A missing or
//! unreadable state starts full, and a refill time in the future (clock skew) counts as now.

use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

/// A token bucket stored at `path`.
#[derive(Debug)]
pub(crate) struct SharedBucket {
    path: PathBuf,
}

impl SharedBucket {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Takes a token if one is free, or returns how long until one will be.
    pub(crate) fn try_take(
        &self,
        interval: Duration,
        burst: f64,
    ) -> io::Result<Result<(), Duration>> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)?;
        // Released when `file` is dropped, even on an early return.
        file.lock()?;

        let mut text = String::new();
        file.read_to_string(&mut text)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let (tokens, refilled) = parse(&text).unwrap_or((burst, now));
        let earned = (now - refilled).max(0.0) / interval.as_secs_f64();
        let mut tokens = (tokens + earned).min(burst);

        let taken = if tokens >= 1.0 {
            tokens -= 1.0;
            Ok(())
        } else {
            Err(interval.mul_f64(1.0 - tokens))
        };

        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        writeln!(file, "{tokens} {now}")?;
        Ok(taken)
    }
}

/// Reads `<tokens> <refilled>`, or `None` if the text isn't that.
fn parse(text: &str) -> Option<(f64, f64)> {
    let mut parts = text.split_whitespace();
    let tokens = parts.next()?.parse::<f64>().ok()?;
    let refilled = parts.next()?.parse::<f64>().ok()?;
    let valid = tokens.is_finite() && tokens >= 0.0 && refilled.is_finite();
    (valid && parts.next().is_none()).then_some((tokens, refilled))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::cache::tests::temp_dir;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn two_buckets_on_one_file_share_the_tokens() {
        let path = temp_dir("shared").join("rate-limit");
        let a = SharedBucket::new(path.clone());
        let b = SharedBucket::new(path);
        assert_eq!(a.try_take(SECOND, 2.0).unwrap(), Ok(()));
        assert_eq!(b.try_take(SECOND, 2.0).unwrap(), Ok(()));
        // The burst is spent, whichever bucket asks.
        assert!(a.try_take(SECOND, 2.0).unwrap().is_err());
        assert!(b.try_take(SECOND, 2.0).unwrap().is_err());
    }

    #[test]
    fn the_wait_is_the_time_to_earn_a_token() {
        let path = temp_dir("wait").join("rate-limit");
        let bucket = SharedBucket::new(path);
        bucket.try_take(SECOND, 1.0).unwrap().unwrap();
        let wait = bucket.try_take(SECOND, 1.0).unwrap().unwrap_err();
        assert!(wait <= SECOND && wait > SECOND / 2, "{wait:?}");
    }

    #[test]
    fn tokens_are_earned_back_over_time() {
        let dir = temp_dir("earn");
        let path = dir.join("rate-limit");
        std::fs::create_dir_all(&dir).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        // Empty two seconds ago, at one token a second.
        std::fs::write(&path, format!("0 {}\n", now - 2.0)).unwrap();
        let bucket = SharedBucket::new(path);
        assert_eq!(bucket.try_take(SECOND, 5.0).unwrap(), Ok(()));
        assert_eq!(bucket.try_take(SECOND, 5.0).unwrap(), Ok(()));
        assert!(bucket.try_take(SECOND, 5.0).unwrap().is_err());
    }

    #[test]
    fn an_unreadable_state_starts_full() {
        let dir = temp_dir("corrupt");
        let path = dir.join("rate-limit");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&path, "not a bucket").unwrap();
        let bucket = SharedBucket::new(path);
        assert_eq!(bucket.try_take(SECOND, 2.0).unwrap(), Ok(()));
        assert_eq!(bucket.try_take(SECOND, 2.0).unwrap(), Ok(()));
        assert!(bucket.try_take(SECOND, 2.0).unwrap().is_err());
    }

    #[test]
    fn a_refill_time_in_the_future_earns_nothing() {
        let dir = temp_dir("future");
        let path = dir.join("rate-limit");
        std::fs::create_dir_all(&dir).unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs_f64();
        std::fs::write(&path, format!("0 {}\n", now + 3600.0)).unwrap();
        let bucket = SharedBucket::new(path);
        assert!(bucket.try_take(SECOND, 5.0).unwrap().is_err());
    }

    #[test]
    fn parse_rejects_anything_but_two_numbers() {
        assert_eq!(parse("1.5 100\n"), Some((1.5, 100.0)));
        assert_eq!(parse(""), None);
        assert_eq!(parse("1"), None);
        assert_eq!(parse("1 2 3"), None);
        assert_eq!(parse("-1 2"), None);
        assert_eq!(parse("NaN 2"), None);
    }
}

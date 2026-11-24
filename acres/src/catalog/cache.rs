//! The on-disk response cache, with a lifetime per kind of resource (ADR 0016).
//!
//! Each response is one file named by a hash of its URL, the same layout `Api` has always used,
//! so existing caches stay readable. Age comes from the file's modified time.

use std::{
    io::ErrorKind,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use bytes::Bytes;

use crate::CacheError;

/// Where responses are kept, if anywhere.
#[derive(Clone, Debug)]
pub(crate) struct Cache {
    /// `None` turns caching off.
    dir: Option<PathBuf>,
    /// Serve whatever is cached, however old, and never go to the network.
    pub(crate) offline: bool,
}

impl Cache {
    pub(crate) fn new(dir: Option<PathBuf>, offline: bool) -> Self {
        Self { dir, offline }
    }

    fn path(&self, endpoint: &str) -> Option<PathBuf> {
        // `{:?}` quotes the URL. It's kept so files written by earlier versions still match.
        let key = xxhash_rust::xxh3::xxh3_64(format!("{endpoint:?}").as_bytes());
        self.dir.as_ref().map(|dir| dir.join(key.to_string()))
    }

    /// The cached body for `endpoint`, unless there is none or it is older than `lifetime`.
    /// Offline, the lifetime is ignored. A `lifetime` of `None` never expires.
    pub(crate) fn load(
        &self,
        endpoint: &str,
        lifetime: Option<Duration>,
    ) -> Result<Option<Bytes>, CacheError> {
        let Some(path) = self.path(endpoint) else {
            return Ok(None);
        };
        let io = |source| CacheError::Io {
            path: path.clone(),
            source,
        };
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(io(e)),
        };
        if !self.offline
            && let Some(lifetime) = lifetime
        {
            let modified = metadata.modified().map_err(io)?;
            // A file from the future (clock skew) counts as brand new.
            let age = SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default();
            if age > lifetime {
                tracing::debug!(endpoint, ?age, "cache entry expired");
                return Ok(None);
            }
        }
        let data = std::fs::read(&path).map_err(io)?;
        tracing::debug!(endpoint, path = %path.display(), "loaded from cache");
        Ok(Some(data.into()))
    }

    /// Writes `data` for `endpoint`, replacing any older entry.
    pub(crate) fn store(&self, endpoint: &str, data: &Bytes) -> Result<(), CacheError> {
        let Some(path) = self.path(endpoint) else {
            return Ok(());
        };
        let io = |source| CacheError::Io {
            path: path.clone(),
            source,
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        std::fs::write(&path, data).map_err(io)?;
        tracing::debug!(endpoint, path = %path.display(), "stored in cache");
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    /// A fresh, empty directory under the system temp dir for one test.
    pub(crate) fn temp_dir(name: &str) -> PathBuf {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "acres-test-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// Makes the entry for `endpoint` look `age` old.
    pub(crate) fn age_entry(cache: &Cache, endpoint: &str, age: Duration) {
        let path = cache.path(endpoint).unwrap();
        let file = std::fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
    }

    const URL: &str = "https://example.com/api/v1/artworks/4";
    const DAY: Duration = Duration::from_secs(24 * 60 * 60);

    #[test]
    fn a_fresh_entry_is_served() {
        let cache = Cache::new(Some(temp_dir("fresh")), false);
        cache.store(URL, &Bytes::from_static(b"{}")).unwrap();
        assert_eq!(cache.load(URL, Some(DAY)).unwrap().unwrap(), "{}");
    }

    #[test]
    fn an_entry_past_its_lifetime_is_a_miss() {
        let cache = Cache::new(Some(temp_dir("stale")), false);
        cache.store(URL, &Bytes::from_static(b"{}")).unwrap();
        age_entry(&cache, URL, 2 * DAY);
        assert_eq!(cache.load(URL, Some(DAY)).unwrap(), None);
    }

    #[test]
    fn no_lifetime_never_expires() {
        let cache = Cache::new(Some(temp_dir("forever")), false);
        cache.store(URL, &Bytes::from_static(b"{}")).unwrap();
        age_entry(&cache, URL, 400 * DAY);
        assert!(cache.load(URL, None).unwrap().is_some());
    }

    #[test]
    fn offline_ignores_lifetimes() {
        let dir = temp_dir("offline");
        let online = Cache::new(Some(dir.clone()), false);
        online.store(URL, &Bytes::from_static(b"{}")).unwrap();
        age_entry(&online, URL, 2 * DAY);
        let offline = Cache::new(Some(dir), true);
        assert!(offline.load(URL, Some(DAY)).unwrap().is_some());
    }

    #[test]
    fn a_missing_entry_is_a_miss() {
        let cache = Cache::new(Some(temp_dir("missing")), false);
        assert_eq!(cache.load(URL, None).unwrap(), None);
    }

    #[test]
    fn no_dir_caches_nothing() {
        let cache = Cache::new(None, false);
        cache.store(URL, &Bytes::from_static(b"{}")).unwrap();
        assert_eq!(cache.load(URL, None).unwrap(), None);
    }

    #[test]
    fn keys_match_the_old_api_layout() {
        let cache = Cache::new(Some(PathBuf::from("/c")), false);
        let old_key = xxhash_rust::xxh3::xxh3_64(format!("{:?}", URL.to_string()).as_bytes());
        assert_eq!(
            cache.path(URL).unwrap(),
            PathBuf::from("/c").join(old_key.to_string())
        );
    }
}

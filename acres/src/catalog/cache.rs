//! The on-disk response cache, with a lifetime per kind of resource (ADR 0016).
//!
//! Each response is one file, `<dir>/<kind>/<hash>`, where `<kind>` is `pages`, `records` or
//! `images` and `<hash>` is the hex xxh3 hash of the URL. Keeping kinds apart lets a user clear
//! one kind (stale records, say) without losing the images. Age comes from the file's modified
//! time. Writes go to a temporary file that is then renamed, so a crash mid-write never leaves a
//! truncated entry behind, which matters most for images, since they never expire.

use std::{
    io::ErrorKind,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use bytes::Bytes;

use super::Resource;
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

    pub(crate) fn dir(&self) -> Option<&std::path::Path> {
        self.dir.as_deref()
    }

    fn path(&self, endpoint: &str, resource: Resource) -> Option<PathBuf> {
        let kind = match resource {
            Resource::Page => "pages",
            Resource::Record => "records",
            Resource::Image => "images",
        };
        let key = xxhash_rust::xxh3::xxh3_64(endpoint.as_bytes());
        self.dir
            .as_ref()
            .map(|dir| dir.join(kind).join(format!("{key:016x}")))
    }

    /// The path of a fresh entry for `endpoint`, unless there is none or it is older than the
    /// resource's lifetime. Offline, the lifetime is ignored.
    fn fresh(&self, endpoint: &str, resource: Resource) -> Result<Option<PathBuf>, CacheError> {
        let Some(path) = self.path(endpoint, resource) else {
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
            && let Some(lifetime) = resource.lifetime()
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
        Ok(Some(path))
    }

    /// Whether a fresh entry for `endpoint` is cached, which costs one `stat` and no read.
    pub(crate) fn contains(&self, endpoint: &str, resource: Resource) -> Result<bool, CacheError> {
        Ok(self.fresh(endpoint, resource)?.is_some())
    }

    /// The cached body for `endpoint`, unless there is none or it is older than the resource's
    /// lifetime. Offline, the lifetime is ignored.
    pub(crate) fn load(
        &self,
        endpoint: &str,
        resource: Resource,
    ) -> Result<Option<Bytes>, CacheError> {
        let Some(path) = self.fresh(endpoint, resource)? else {
            return Ok(None);
        };
        let data = std::fs::read(&path).map_err(|source| CacheError::Io {
            path: path.clone(),
            source,
        })?;
        tracing::debug!(endpoint, path = %path.display(), "loaded from cache");
        Ok(Some(data.into()))
    }

    /// Writes `data` for `endpoint`, replacing any older entry.
    pub(crate) fn store(
        &self,
        endpoint: &str,
        resource: Resource,
        data: &Bytes,
    ) -> Result<(), CacheError> {
        let Some(path) = self.path(endpoint, resource) else {
            return Ok(());
        };
        let io = |source| CacheError::Io {
            path: path.clone(),
            source,
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        // Unique per process and call, so two writers never share a temporary file. A rename
        // within one directory replaces the old entry in one step.
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let temp = path.with_extension(format!(
            "tmp-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let written = std::fs::write(&temp, data).and_then(|()| std::fs::rename(&temp, &path));
        if let Err(e) = written {
            let _ = std::fs::remove_file(&temp);
            return Err(io(e));
        }
        tracing::debug!(endpoint, path = %path.display(), "stored in cache");
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        sync::atomic::{AtomicU32, Ordering},
        time::Duration,
    };

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
    pub(crate) fn age_entry(cache: &Cache, endpoint: &str, resource: Resource, age: Duration) {
        let path = cache.path(endpoint, resource).unwrap();
        let file = std::fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
    }

    const URL: &str = "https://example.com/api/v1/artworks/4";
    const DAY: Duration = Duration::from_secs(24 * 60 * 60);

    #[test]
    fn a_fresh_entry_is_served() {
        let cache = Cache::new(Some(temp_dir("fresh")), false);
        cache
            .store(URL, Resource::Record, &Bytes::from_static(b"{}"))
            .unwrap();
        assert_eq!(cache.load(URL, Resource::Record).unwrap().unwrap(), "{}");
    }

    #[test]
    fn an_entry_past_its_lifetime_is_a_miss() {
        let cache = Cache::new(Some(temp_dir("stale")), false);
        cache
            .store(URL, Resource::Record, &Bytes::from_static(b"{}"))
            .unwrap();
        age_entry(&cache, URL, Resource::Record, 8 * DAY);
        assert_eq!(cache.load(URL, Resource::Record).unwrap(), None);
    }

    #[test]
    fn images_never_expire() {
        let cache = Cache::new(Some(temp_dir("forever")), false);
        cache
            .store(URL, Resource::Image, &Bytes::from_static(b"{}"))
            .unwrap();
        age_entry(&cache, URL, Resource::Image, 400 * DAY);
        assert!(cache.load(URL, Resource::Image).unwrap().is_some());
    }

    #[test]
    fn offline_ignores_lifetimes() {
        let dir = temp_dir("offline");
        let online = Cache::new(Some(dir.clone()), false);
        online
            .store(URL, Resource::Record, &Bytes::from_static(b"{}"))
            .unwrap();
        age_entry(&online, URL, Resource::Record, 8 * DAY);
        let offline = Cache::new(Some(dir), true);
        assert!(offline.load(URL, Resource::Record).unwrap().is_some());
    }

    #[test]
    fn a_missing_entry_is_a_miss() {
        let cache = Cache::new(Some(temp_dir("missing")), false);
        assert_eq!(cache.load(URL, Resource::Record).unwrap(), None);
        assert!(!cache.contains(URL, Resource::Record).unwrap());
    }

    #[test]
    fn contains_matches_load() {
        let cache = Cache::new(Some(temp_dir("contains")), false);
        cache
            .store(URL, Resource::Record, &Bytes::from_static(b"{}"))
            .unwrap();
        assert!(cache.contains(URL, Resource::Record).unwrap());
        assert!(!cache.contains(URL, Resource::Image).unwrap());
        age_entry(&cache, URL, Resource::Record, 8 * DAY);
        assert!(!cache.contains(URL, Resource::Record).unwrap(), "expired");
        assert!(
            !Cache::new(None, false)
                .contains(URL, Resource::Record)
                .unwrap()
        );
    }

    #[test]
    fn no_dir_caches_nothing() {
        let cache = Cache::new(None, false);
        cache
            .store(URL, Resource::Record, &Bytes::from_static(b"{}"))
            .unwrap();
        assert_eq!(cache.load(URL, Resource::Record).unwrap(), None);
    }

    #[test]
    fn kinds_are_kept_apart() {
        let cache = Cache::new(Some(temp_dir("kinds")), false);
        cache
            .store(URL, Resource::Record, &Bytes::from_static(b"record"))
            .unwrap();
        cache
            .store(URL, Resource::Page, &Bytes::from_static(b"page"))
            .unwrap();
        assert_eq!(
            cache.load(URL, Resource::Record).unwrap().unwrap(),
            "record"
        );
        assert_eq!(cache.load(URL, Resource::Page).unwrap().unwrap(), "page");
        assert_eq!(cache.load(URL, Resource::Image).unwrap(), None);
    }

    #[test]
    fn entries_live_under_their_kind() {
        let cache = Cache::new(Some(PathBuf::from("/c")), false);
        let path = cache.path(URL, Resource::Image).unwrap();
        assert_eq!(path.parent().unwrap(), PathBuf::from("/c/images"));
        assert_eq!(path.file_name().unwrap().len(), 16);
    }

    #[test]
    fn a_store_leaves_no_temporary_files() {
        let dir = temp_dir("temp");
        let cache = Cache::new(Some(dir.clone()), false);
        cache
            .store(URL, Resource::Record, &Bytes::from_static(b"a"))
            .unwrap();
        cache
            .store(URL, Resource::Record, &Bytes::from_static(b"b"))
            .unwrap();
        let files: Vec<_> = std::fs::read_dir(dir.join("records")).unwrap().collect();
        assert_eq!(files.len(), 1);
        assert_eq!(cache.load(URL, Resource::Record).unwrap().unwrap(), "b");
    }
}

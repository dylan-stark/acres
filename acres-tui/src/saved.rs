//! The saved list: artworks marked with `m`, each with when it was saved and an optional note.
//!
//! It lives in the data directory as `saved.json`, a format of the TUI's own (ADR 0013):
//!
//! ```json
//! {"version": 1, "saved": [{"record": {"id": 27992, "title": "…"},
//!   "config": {"iiif_url": "…", "website_url": "…"},
//!   "saved_at": "2026-11-25T18:37:10Z", "note": "See it in person"}]}
//! ```
//!
//! Each entry keeps the record as fetched and the config its page came with, so the list opens
//! offline and its images load from the cache. `--from-file` reads it as well as an `Artworks`
//! JSON.

use std::{
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use acres::{ApiConfig, artworks::ArtworkSummary};
use color_eyre::eyre::{self, WrapErr};
use serde::{Deserialize, Serialize};

use crate::entry::Entry;

/// The `saved.json` version this build writes, and the newest it reads.
pub const VERSION: u32 = 1;

/// The file name in the data directory.
pub const FILE_NAME: &str = "saved.json";

/// One saved artwork.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    /// The record as it was listed.
    pub record: ArtworkSummary,
    /// The image server and website the record came with.
    #[serde(default)]
    pub config: ApiConfig,
    /// When it was saved, in RFC 3339 UTC, such as `2026-11-25T18:37:10Z`.
    pub saved_at: String,
    /// A one-line note, if one was written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Saved {
    fn entry(&self) -> Entry {
        Entry {
            summary: self.record.clone(),
            config: self.config.clone(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SavedFile {
    version: u32,
    saved: Vec<Saved>,
}

/// The saved artworks, newest first, and the file they are kept in.
#[derive(Debug, Default)]
pub struct SavedList {
    /// Where changes are written, or `None` to keep them in memory, as tests do.
    path: Option<PathBuf>,
    items: Vec<Saved>,
}

impl SavedList {
    /// A list that is never written, for tests and benches.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// The list in `path`, empty if the file does not exist yet.
    pub fn open(path: impl Into<PathBuf>) -> eyre::Result<Self> {
        let path = path.into();
        let items = match std::fs::read_to_string(&path) {
            Ok(text) => {
                parse(&text).wrap_err_with(|| format!("could not read {}", path.display()))?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e).wrap_err_with(|| format!("could not read {}", path.display())),
        };
        Ok(Self {
            path: Some(path),
            items,
        })
    }

    /// The saved artworks as list entries, newest first.
    pub fn entries(&self) -> Vec<Entry> {
        self.items.iter().map(Saved::entry).collect()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn contains(&self, id: u32) -> bool {
        self.items.iter().any(|saved| saved.record.id == id)
    }

    /// The note on artwork `id`, if it is saved and has one.
    pub fn note(&self, id: u32) -> Option<&str> {
        self.items
            .iter()
            .find(|saved| saved.record.id == id)?
            .note
            .as_deref()
    }

    /// Saves `entry`, or unsaves it if it was saved, and returns whether it is saved now.
    pub fn toggle(&mut self, entry: &Entry) -> eyre::Result<bool> {
        let id = entry.id();
        let saved = if let Some(at) = self.items.iter().position(|s| s.record.id == id) {
            self.items.remove(at);
            false
        } else {
            self.items.insert(
                0,
                Saved {
                    record: entry.summary.clone(),
                    config: entry.config.clone(),
                    saved_at: now_rfc3339(),
                    note: None,
                },
            );
            true
        };
        self.write()?;
        Ok(saved)
    }

    /// Sets the note on saved artwork `id`; an empty or blank note removes it. Does nothing if
    /// `id` is not saved.
    pub fn set_note(&mut self, id: u32, note: &str) -> eyre::Result<()> {
        let Some(saved) = self.items.iter_mut().find(|s| s.record.id == id) else {
            return Ok(());
        };
        let note = note.trim();
        saved.note = (!note.is_empty()).then(|| note.to_string());
        self.write()
    }

    /// Writes the list to its file, through a temporary file so a crash never leaves half of one.
    fn write(&self) -> eyre::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        let file = SavedFile {
            version: VERSION,
            saved: self.items.clone(),
        };
        let json = serde_json::to_vec_pretty(&file)?;
        write_atomically(path, &json)
            .wrap_err_with(|| format!("could not write {}", path.display()))
    }
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension("json.tmp");
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(&temporary, path)
}

fn parse(text: &str) -> eyre::Result<Vec<Saved>> {
    let file: SavedFile = serde_json::from_str(text)?;
    if file.version > VERSION {
        eyre::bail!(
            "saved list version {} is newer than this acres-tui reads ({VERSION})",
            file.version
        );
    }
    Ok(file.saved)
}

/// Whether `text` looks like a saved list rather than an `Artworks` JSON.
pub fn is_saved_list(text: &str) -> bool {
    #[derive(Deserialize)]
    struct Probe {
        #[allow(dead_code)]
        version: u32,
        #[allow(dead_code)]
        saved: serde::de::IgnoredAny,
    }
    serde_json::from_str::<Probe>(text).is_ok()
}

/// The entries of a saved list's text, newest first, for `--from-file`.
pub fn entries_from(text: &str) -> eyre::Result<Vec<Entry>> {
    Ok(parse(text)?.iter().map(Saved::entry).collect())
}

/// Now, in RFC 3339 UTC to the second.
fn now_rfc3339() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    rfc3339(seconds)
}

/// `seconds` since the Unix epoch, in RFC 3339 UTC, such as `2026-11-25T18:37:10Z`.
fn rfc3339(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let rest = seconds % 86_400;
    // Days to a civil date, from Howard Hinnant's `civil_from_days`.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u32) -> Entry {
        Entry {
            summary: ArtworkSummary {
                id,
                title: format!("Artwork {id}"),
                image_id: Some(format!("image-{id}")),
                ..ArtworkSummary::default()
            },
            config: ApiConfig {
                iiif_url: "https://www.artic.edu/iiif/2".into(),
                website_url: "https://www.artic.edu".into(),
            },
        }
    }

    fn temp_file(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("acres-tui-saved-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join(FILE_NAME)
    }

    #[test]
    fn rfc3339_formats_utc_seconds() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_795_632_630), "2026-11-25T18:50:30Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    }

    #[test]
    fn toggling_saves_newest_first_and_unsaves() {
        let mut list = SavedList::in_memory();
        assert!(list.toggle(&entry(1)).unwrap());
        assert!(list.toggle(&entry(2)).unwrap());
        let ids: Vec<u32> = list.entries().iter().map(Entry::id).collect();
        assert_eq!(ids, [2, 1]);
        assert!(!list.toggle(&entry(1)).unwrap());
        assert!(!list.contains(1));
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn notes_are_trimmed_and_blank_ones_removed() {
        let mut list = SavedList::in_memory();
        list.toggle(&entry(1)).unwrap();
        list.set_note(1, "  see it in person ").unwrap();
        assert_eq!(list.note(1), Some("see it in person"));
        list.set_note(1, "   ").unwrap();
        assert_eq!(list.note(1), None);
        list.set_note(7, "not saved").unwrap();
        assert_eq!(list.note(7), None);
    }

    #[test]
    fn the_file_round_trips_and_reads_as_a_saved_list() {
        let path = temp_file("round-trip");
        let mut list = SavedList::open(&path).unwrap();
        assert!(list.is_empty(), "no file yet");
        list.toggle(&entry(27992)).unwrap();
        list.set_note(27992, "dots").unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(is_saved_list(&text));
        assert!(text.contains("\"version\": 1"));
        let again = SavedList::open(&path).unwrap();
        assert_eq!(again.note(27992), Some("dots"));
        assert_eq!(again.entries(), list.entries());
        assert_eq!(entries_from(&text).unwrap(), list.entries());
    }

    #[test]
    fn an_artworks_file_is_not_a_saved_list() {
        let artworks = crate::components::test_support::ARTWORKS_FIXTURE;
        assert!(!is_saved_list(artworks));
    }

    #[test]
    fn a_newer_version_is_refused() {
        let error = entries_from(r#"{"version": 2, "saved": []}"#).unwrap_err();
        assert!(error.to_string().contains("newer"), "{error}");
    }
}

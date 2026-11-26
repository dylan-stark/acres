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
//!
//! An entry may also keep named views of its art (ADR 0017), newest first, which the file holds
//! without a new version:
//!
//! ```json
//! "views": [{"name": "Fuji", "center": [0.62, 0.41], "zoom": 3.2, "base": "fill",
//!            "frame": [20, 12], "saved_at": "2026-11-25T20:41:12Z"}]
//! ```

use std::{
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use acres::{ApiConfig, artworks::ArtworkSummary};
use color_eyre::eyre::{self, WrapErr};
use serde::{Deserialize, Serialize};

use crate::{
    components::art_view::{ArtView, Base, FrameSize},
    entry::Entry,
};

/// The `saved.json` version this build writes, and the newest it reads.
pub const VERSION: u32 = 1;

/// The file name in the data directory.
pub const FILE_NAME: &str = "saved.json";

/// What a saved view is zoomed relative to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SavedBase {
    Fit,
    Fill,
}

/// A named view of a saved artwork's art.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedView {
    pub name: String,
    /// The view's center, in fractions of the image.
    pub center: [f64; 2],
    /// The zoom relative to `base`.
    pub zoom: f64,
    pub base: SavedBase,
    /// `[cols, rows]` of a custom frame; absent for the pane or the screen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<[u16; 2]>,
    pub saved_at: String,
}

impl SavedView {
    /// A view named `name` of `view` on `frame`, saved now.
    pub fn new(name: &str, view: ArtView, frame: Option<FrameSize>) -> Self {
        Self {
            name: name.to_string(),
            center: [view.center.0, view.center.1],
            zoom: view.zoom,
            base: match view.base {
                Base::Fit => SavedBase::Fit,
                Base::Fill => SavedBase::Fill,
            },
            frame: match frame {
                Some(FrameSize::Custom { cols, rows }) => Some([cols, rows]),
                _ => None,
            },
            saved_at: now_rfc3339(),
        }
    }

    /// The view to show, and its frame, or `None` for the default frame.
    pub fn view(&self) -> (ArtView, Option<FrameSize>) {
        let finite = |v: f64, fallback: f64| if v.is_finite() { v } else { fallback };
        let view = ArtView {
            base: match self.base {
                SavedBase::Fit => Base::Fit,
                SavedBase::Fill => Base::Fill,
            },
            zoom: finite(self.zoom, 1.0).max(f64::MIN_POSITIVE),
            center: (
                finite(self.center[0], 0.5).clamp(0.0, 1.0),
                finite(self.center[1], 0.5).clamp(0.0, 1.0),
            ),
        };
        let frame = self
            .frame
            .filter(|[cols, rows]| *cols > 0 && *rows > 0)
            .map(|[cols, rows]| FrameSize::Custom { cols, rows });
        (view, frame)
    }
}

/// One saved artwork.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Named views of its art, newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub views: Vec<SavedView>,
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
                    views: Vec::new(),
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

    /// The views saved for artwork `id`, newest first.
    pub fn views(&self, id: u32) -> &[SavedView] {
        self.items
            .iter()
            .find(|saved| saved.record.id == id)
            .map_or(&[], |saved| saved.views.as_slice())
    }

    /// Saves `view` for `entry`, newest first, saving the artwork too if it is not yet.
    pub fn add_view(&mut self, entry: &Entry, view: SavedView) -> eyre::Result<()> {
        if !self.contains(entry.id()) {
            self.toggle(entry)?;
        }
        if let Some(saved) = self.items.iter_mut().find(|s| s.record.id == entry.id()) {
            saved.views.insert(0, view);
        }
        self.write()
    }

    /// Renames view `at` of artwork `id`; a blank name keeps the old one.
    pub fn rename_view(&mut self, id: u32, at: usize, name: &str) -> eyre::Result<()> {
        let name = name.trim();
        let view = self
            .items
            .iter_mut()
            .find(|s| s.record.id == id)
            .and_then(|saved| saved.views.get_mut(at));
        match view {
            Some(view) if !name.is_empty() => view.name = name.to_string(),
            _ => return Ok(()),
        }
        self.write()
    }

    /// Deletes view `at` of artwork `id`.
    pub fn delete_view(&mut self, id: u32, at: usize) -> eyre::Result<()> {
        let Some(saved) = self.items.iter_mut().find(|s| s.record.id == id) else {
            return Ok(());
        };
        if at < saved.views.len() {
            saved.views.remove(at);
            self.write()?;
        }
        Ok(())
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
pub fn now_rfc3339() -> String {
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
    fn views_save_newest_first_and_save_the_artwork_too() {
        let path = temp_file("views");
        let mut list = SavedList::open(&path).unwrap();
        let fill = ArtView {
            zoom: 3.2,
            center: (0.62, 0.41),
            ..ArtView::fill()
        };
        let stamp = Some(FrameSize::Custom { cols: 20, rows: 12 });
        list.add_view(&entry(24645), SavedView::new("Fuji", fill, stamp))
            .unwrap();
        assert!(list.contains(24645), "saving a view saves the artwork");
        list.add_view(
            &entry(24645),
            SavedView::new("whole", ArtView::default(), None),
        )
        .unwrap();
        let names: Vec<&str> = list.views(24645).iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["whole", "Fuji"]);
        assert_eq!(list.views(24645)[1].view(), (fill, stamp));

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(
            text.contains("\"version\": 1"),
            "the file stays at version 1"
        );
        assert!(text.contains("\"base\": \"fill\""));
        assert!(text.contains("\"frame\": [\n"));
        let mut again = SavedList::open(&path).unwrap();
        assert_eq!(again.views(24645), list.views(24645));

        again.rename_view(24645, 1, " Mount Fuji ").unwrap();
        again.rename_view(24645, 1, "  ").unwrap();
        assert_eq!(again.views(24645)[1].name, "Mount Fuji");
        again.delete_view(24645, 0).unwrap();
        again.delete_view(24645, 9).unwrap();
        let reread = SavedList::open(&path).unwrap();
        assert_eq!(reread.views(24645).len(), 1);
        assert_eq!(reread.views(24645)[0].name, "Mount Fuji");
    }

    #[test]
    fn a_saved_entry_without_views_reads_and_writes_as_before() {
        let text = r#"{"version": 1, "saved": [{"record": {"id": 1}, "saved_at": "2026-11-25T18:37:10Z"}]}"#;
        let saved = parse(text).unwrap();
        assert!(saved[0].views.is_empty());
        let json = serde_json::to_string(&saved[0]).unwrap();
        assert!(!json.contains("views"));
    }

    #[test]
    fn a_hand_edited_view_comes_back_in_range() {
        let view = SavedView {
            name: "odd".into(),
            center: [3.0, -1.0],
            zoom: f64::NAN,
            base: SavedBase::Fit,
            frame: Some([0, 12]),
            saved_at: String::new(),
        };
        let (art, frame) = view.view();
        assert_eq!((art.center, art.zoom, frame), ((1.0, 0.0), 1.0, None));
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

//! The explorer screen: the search bar, results, art and record panes, the status bar, and the
//! overlays drawn over them (ADR 0013).
//!
//! The explorer owns focus. A key is looked up in the focused pane's context, then in `Global`,
//! and the action it maps to is sent like any other. Actions that change what is selected come
//! back as [`Action::ShowArtwork`], which the image fetcher picks up.
//!
//! Live, the explorer starts on the Highlights and runs searches through the catalog fetcher.
//! Each search gets a new generation, and a list that comes back for an older one is dropped,
//! so only the latest search ever reaches the screen. From a file, the search bar narrows the
//! loaded list by title instead.
//!
//! Every new list, from a search, a pivot (`A`, `D`) or a surprise (`R`), first pushes the one
//! showing onto a history that `backspace` pops, with its cursor and pages. `'` swaps the list
//! for the saved artworks, which `m` adds to and removes from and `e` notes.

use acres::artworks::request::search::MAX_RESULTS;
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    components::{
        Component,
        catalog::{Listing, Load, Loaded, Query, Sort, default_filters},
        filters::FilterPanel,
        frames::Frames,
        help::Help,
        home::Home,
        hud::Hud,
        logs::Logs,
        note::NoteEditor,
        pane_block,
        quit::QuitPrompt,
        ramps::Ramps,
        record::Record,
        results::Results,
        search::Search,
        status::Status,
    },
    config::{Config, Context, KeyBindings, key_event_to_string, normalize},
    entry::Entry,
    saved::SavedList,
};

/// Where the list comes from.
#[derive(Debug, Clone)]
pub enum Source {
    /// A saved `Artworks` file: searching narrows it by title.
    File(Vec<Entry>),
    /// The live catalog, starting on these Highlights.
    Live { highlights: Vec<u32> },
}

/// Start loading the next page when the cursor is this close to the end of the list.
const LOAD_AHEAD: usize = 10;

/// The most earlier lists `backspace` can go back through.
const HISTORY: usize = 50;

/// What the live list holds and what is on its way.
#[derive(Debug, Default)]
struct Live {
    highlights: Vec<u32>,
    /// The current search; increases with each new one.
    generation: u64,
    /// The query the list is for, or `None` for the Highlights.
    query: Option<Query>,
    /// The filters and sort the next search uses, with the text last searched.
    settings: Query,
    /// Records matching, across all pages.
    total: u64,
    /// The next page to ask for, from 1.
    next_page: u32,
    /// Whether a list request is in flight.
    loading: bool,
    /// Whether the Highlights were unavailable and the list is the fallback instead.
    fallback: bool,
    /// Why the last request failed, until the next one.
    error: Option<String>,
    rate_limited: bool,
    /// Whether the list is one surprise artwork, which has no more pages.
    surprise: bool,
    /// Whether the first page of a new list is on its way, so the list showing is already in
    /// the history.
    replacing: bool,
}

/// A live list as it was, for going back to it.
#[derive(Clone)]
struct Snapshot {
    results: Results,
    query: Option<Query>,
    settings: Query,
    total: u64,
    next_page: u32,
    fallback: bool,
    surprise: bool,
}

/// Which list the results pane shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    Results,
    Saved,
}

/// A pane that can take focus, in `tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Search,
    Results,
    Art,
    Record,
}

impl Pane {
    const ALL: [Pane; 4] = [Pane::Search, Pane::Results, Pane::Art, Pane::Record];

    fn context(self) -> Context {
        match self {
            Pane::Search => Context::Search,
            Pane::Results => Context::List,
            Pane::Art => Context::Art,
            Pane::Record => Context::Record,
        }
    }

    /// The pane `delta` steps along in `tab` order, wrapping around.
    fn step(self, delta: isize) -> Pane {
        let at = Pane::ALL.iter().position(|&p| p == self).unwrap_or(0);
        let len = Pane::ALL.len() as isize;
        Pane::ALL[(at as isize + delta).rem_euclid(len) as usize]
    }
}

/// An overlay drawn over the panes, which takes keys while it is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Help,
    Ramps,
    /// Asks before quitting.
    Quit,
    /// The full record of the selected artwork.
    Record,
    /// The search's filters.
    Filters,
    /// Edits a saved artwork's note.
    Note,
    /// Picks the art's frame size.
    Frames,
}

/// Where the last draw put each pane, for mapping mouse events.
#[derive(Debug, Default, Clone, Copy)]
struct Areas {
    search: Rect,
    results: Rect,
    art: Rect,
    record: Rect,
}

impl Areas {
    fn pane_at(&self, at: Position) -> Option<Pane> {
        [
            (self.search, Pane::Search),
            (self.results, Pane::Results),
            (self.art, Pane::Art),
            (self.record, Pane::Record),
        ]
        .into_iter()
        .find(|(area, _)| area.contains(at))
        .map(|(_, pane)| pane)
    }
}

pub struct Explorer {
    keybindings: KeyBindings,
    show_keys: bool,
    focus: Pane,
    overlay: Option<Overlay>,
    zen: bool,
    /// The last key and what it did, for the status bar.
    echo: Option<String>,
    search: Search,
    /// The list showing, and the other one: the results while the saved list shows, or the
    /// other way around.
    results: Results,
    other: Results,
    view: View,
    history: Vec<Snapshot>,
    saved: SavedList,
    note: NoteEditor,
    /// The state of the random numbers for surprises.
    seed: u64,
    art: Home,
    record: Record,
    full_record: Record,
    filters: FilterPanel,
    /// Whether the departments and types were asked for.
    facets_requested: bool,
    hud: Hud,
    logs: Logs,
    ramps: Ramps,
    frames: Frames,
    /// Where a drag over the art last was, while one is under way.
    drag: Option<Position>,
    quit: QuitPrompt,
    help: Help,
    status: Status,
    areas: Areas,
    /// The terminal's size at the last draw.
    screen: (u16, u16),
    /// Set when the list comes from the catalog.
    live: Option<Live>,
    action_tx: Option<UnboundedSender<Action>>,
}

impl Explorer {
    /// An explorer over `source`, with the results focused.
    pub fn new(source: Source, config: &Config) -> Self {
        Self::with_art(source, config, Home::new())
    }

    /// The same, drawing the art with `art`, so tests and benches can fix the color depth.
    pub fn with_art(source: Source, config: &Config, art: Home) -> Self {
        let (results, live) = match source {
            Source::File(entries) => (Results::new(entries), None),
            Source::Live { highlights } => (
                Results::default(),
                Some(Live {
                    highlights,
                    ..Live::default()
                }),
            ),
        };
        let mut record = Record::default();
        record.show(results.selected().cloned());
        Self {
            keybindings: config.keybindings.clone(),
            show_keys: config.config.show_keys,
            focus: Pane::Results,
            overlay: None,
            zen: false,
            echo: None,
            search: Search::default(),
            results,
            other: Results::default(),
            view: View::Results,
            history: Vec::new(),
            saved: SavedList::in_memory(),
            note: NoteEditor::default(),
            seed: clock_seed(),
            art,
            record,
            full_record: Record::overlay(),
            filters: FilterPanel::default(),
            facets_requested: false,
            hud: Hud::new(),
            logs: Logs::new(),
            ramps: Ramps::new(),
            frames: Frames::default(),
            drag: None,
            quit: QuitPrompt::default(),
            help: Help::default(),
            status: Status::default(),
            areas: Areas::default(),
            screen: (0, 0),
            live,
            action_tx: None,
        }
    }

    /// Keeps saved artworks in `saved` instead of in memory.
    pub fn with_saved(mut self, saved: SavedList) -> Self {
        self.saved = saved;
        self.refresh_saved();
        self
    }

    pub fn view(&self) -> View {
        self.view
    }

    /// The artwork selected now.
    pub fn selected(&self) -> Option<&Entry> {
        self.results.selected()
    }

    pub fn focus(&self) -> Pane {
        self.focus
    }

    pub fn overlay(&self) -> Option<Overlay> {
        self.overlay
    }

    pub fn zen(&self) -> bool {
        self.zen
    }

    /// Where keys are looked up first.
    fn context(&self) -> Context {
        match self.overlay {
            Some(_) => Context::Overlay,
            None => self.pane_context(),
        }
    }

    /// The context of the focused pane, or the art in zen, which shows only the art.
    fn pane_context(&self) -> Context {
        if self.zen {
            Context::Art
        } else {
            self.focus.context()
        }
    }

    fn lookup(&self, context: Context, key: KeyEvent) -> Option<&Action> {
        self.keybindings.get(&context)?.get(&vec![key])
    }

    /// The first key bound to `action`, looked up in `context` and then `Global`.
    fn key_for(&self, context: Context, action: &Action) -> Option<String> {
        [context, Context::Global].into_iter().find_map(|context| {
            let mut keys: Vec<String> = self
                .keybindings
                .get(&context)?
                .iter()
                .filter(|(_, bound)| *bound == action)
                .map(|(keys, _)| keys.iter().map(key_event_to_string).collect::<String>())
                .collect();
            keys.sort_by_key(|key| (key.chars().count() > 1, key.clone()));
            keys.into_iter().next()
        })
    }

    /// Every key bound to `action` in `context`, joined like `q, enter`, shortest first.
    fn keys_for(&self, context: Context, action: &Action) -> String {
        let mut keys: Vec<String> = self
            .keybindings
            .get(&context)
            .into_iter()
            .flatten()
            .filter(|(_, bound)| *bound == action)
            .map(|(keys, _)| keys.iter().map(key_event_to_string).collect::<String>())
            .collect();
        keys.sort_by_key(|key| (key.chars().count() > 1, key.clone()));
        keys.join(", ")
    }

    /// Opens the quit prompt, naming the keys that answer it.
    fn ask_quit(&mut self) {
        let quit = [
            self.keys_for(Context::Global, &Action::AskQuit),
            self.keys_for(Context::Overlay, &Action::Select),
        ];
        self.quit.quit_keys = quit
            .into_iter()
            .filter(|keys| !keys.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
        self.quit.stay_keys = self.keys_for(Context::Overlay, &Action::Cancel);
        if self.quit.stay_keys.is_empty() {
            self.quit.stay_keys = self.keys_for(Context::Global, &Action::Cancel);
        }
        self.overlay = Some(Overlay::Quit);
    }

    /// The keys the status bar suggests for where focus is.
    fn hints(&self) -> Vec<(String, &'static str)> {
        let context = self.context();
        let actions: &[Action] = match (self.overlay, self.focus) {
            (Some(Overlay::Help), _) => &[Action::MoveDown, Action::Cancel],
            (Some(Overlay::Ramps), _) => &[Action::Select, Action::Cancel],
            // The prompt names its own keys.
            (Some(Overlay::Quit), _) => &[],
            (Some(Overlay::Filters), _) => &[],
            (Some(Overlay::Note), _) => &[],
            (Some(Overlay::Frames), _) => &[Action::Select, Action::Cancel],
            (Some(Overlay::Record), _) => &[Action::MoveDown, Action::NextArtwork, Action::Cancel],
            (None, Pane::Search) => &[Action::RunSearch, Action::EarlierQuery, Action::Cancel],
            (None, Pane::Results) if self.view == View::Saved => {
                &[Action::MoveDown, Action::EditNote, Action::SwitchSaved]
            }
            (None, Pane::Results) => {
                &[Action::MoveDown, Action::ShowFullImage, Action::FocusSearch]
            }
            (None, Pane::Art) => &[
                Action::ShowFullImage,
                Action::ZoomIn,
                Action::FillArt,
                Action::PickFrame,
                Action::ToggleZen,
            ],
            (None, Pane::Record) => &[
                Action::ToggleFullRecord,
                Action::OpenRecord,
                Action::MoveDown,
            ],
        };
        std::iter::once(&Action::ToggleHelp)
            .chain(actions)
            .filter_map(|action| {
                let key = self.key_for(context, action)?;
                let description = match action {
                    Action::ToggleHelp => "help",
                    _ => action.description()?,
                };
                Some((key, description))
            })
            .collect()
    }

    fn set_echo(&mut self, key: KeyEvent, action: Option<&Action>) {
        if !self.show_keys {
            return;
        }
        let description = action
            .and_then(Action::description)
            .unwrap_or("not mapped here");
        self.echo = Some(format!("{} · {description}", key_event_to_string(&key)));
    }

    /// Shows `artwork` if the selection moved to it, and asks for its image. Near the end of a
    /// live list, also asks for the next page.
    fn preview(&mut self, artwork: Option<Entry>) -> Option<Action> {
        self.load_more();
        let artwork = artwork?;
        self.show_record(Some(artwork.clone()));
        if self.overlay == Some(Overlay::Record) {
            self.full_record.show(Some(artwork.clone()));
            self.fetch_full_record();
        }
        Some(Action::ShowArtwork(artwork))
    }

    /// Shows `artwork` in the record pane, with its note if it is saved.
    fn show_record(&mut self, artwork: Option<Entry>) {
        let id = artwork.as_ref().map(Entry::id);
        self.record.show(artwork);
        self.record.saved = id.is_some_and(|id| self.saved.contains(id));
        self.record.note = id.and_then(|id| self.saved.note(id)).map(String::from);
    }

    /// Marks the saved artworks in both lists and shows the selected one's note.
    fn refresh_saved(&mut self) {
        let ids: Vec<u32> = self.saved.entries().iter().map(Entry::id).collect();
        self.results.mark(ids.iter().copied());
        self.other.mark(ids);
        self.show_record(self.results.selected().cloned());
    }

    /// A random number for the next surprise (xorshift64*).
    fn random(&mut self) -> u64 {
        let mut x = self.seed;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.seed = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    /// Asks for the overlay's full record, if it still needs one.
    fn fetch_full_record(&mut self) {
        if let Some(id) = self.full_record.wants_full() {
            self.send(Action::FetchRecord(id));
        }
    }

    /// The selected artwork's page on artic.edu.
    fn link(&self) -> Option<String> {
        Some(self.results.selected()?.link())
    }

    fn send(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    /// Starts a new live search for `text` with the current filters and sort, or the
    /// Highlights when there is nothing to search for.
    fn search_live(&mut self, text: String) {
        let Some(live) = &self.live else { return };
        let settings = Query {
            text,
            ..live.settings.clone()
        };
        self.run_live(settings);
    }

    /// Starts a new live list with `settings`, remembering the one showing for `backspace`.
    fn run_live(&mut self, settings: Query) {
        self.show_results();
        self.remember();
        let Some(live) = &mut self.live else { return };
        live.settings = settings;
        live.generation += 1;
        live.loading = true;
        live.replacing = true;
        live.surprise = false;
        live.error = None;
        let load = if live.settings.is_empty() {
            live.query = None;
            Load::Highlights(live.highlights.clone())
        } else {
            live.query = Some(live.settings.clone());
            Load::Search {
                query: live.settings.clone(),
                page: 1,
            }
        };
        let generation = live.generation;
        self.send(Action::Load { generation, load });
    }

    /// Asks for one random artwork matching the search and filters, or picks one from the
    /// file's list.
    fn surprise(&mut self) -> Option<Action> {
        let seed = self.random();
        if self.live.is_none() || self.view == View::Saved {
            let (shown, _) = self.results.counts();
            if shown == 0 {
                return None;
            }
            let to = usize::try_from(seed % shown as u64).unwrap_or(0);
            let delta = to as isize - self.results.selected_row() as isize;
            let moved = self.results.step(delta);
            return self.preview(moved);
        }
        // Another surprise replaces one, rather than filling the history with them.
        if !self.live.as_ref().is_some_and(|live| live.surprise) {
            self.remember();
        }
        let live = self.live.as_mut()?;
        live.generation += 1;
        live.loading = true;
        live.replacing = true;
        live.surprise = true;
        live.error = None;
        let query = live.settings.clone();
        live.query = Some(query.clone());
        let generation = live.generation;
        self.send(Action::Load {
            generation,
            load: Load::Surprise { query, seed },
        });
        None
    }

    /// Lists more like the selected artwork: by its artist, or from its department.
    fn pivot(&mut self, by_artist: bool) {
        let what = if by_artist { "artist" } else { "department" };
        if self.live.is_none() {
            self.echo = Some(format!(
                "more by {what} needs a live search, not --from-file"
            ));
            return;
        }
        let Some(summary) = self.results.selected().map(|e| e.summary.clone()) else {
            return;
        };
        let Some(live) = &self.live else { return };
        let query = match (by_artist, summary.artist_id, summary.department_id) {
            (true, Some(id), _) => live.settings.by_artist(id, summary.artist_title),
            (false, _, Some(id)) => live.settings.from_department(id, summary.department_title),
            _ => {
                self.echo = Some(format!("this artwork has no {what} on record"));
                return;
            }
        };
        self.search.clear();
        self.focus = Pane::Results;
        self.run_live(query);
    }

    /// Pushes the live list showing onto the history, unless it is already there.
    fn remember(&mut self) {
        let Some(live) = &self.live else { return };
        if self.view != View::Results || live.replacing || self.results.counts().1 == 0 {
            return;
        }
        if self.history.len() == HISTORY {
            self.history.remove(0);
        }
        self.history.push(Snapshot {
            results: self.results.clone(),
            query: live.query.clone(),
            settings: live.settings.clone(),
            total: live.total,
            next_page: live.next_page,
            fallback: live.fallback,
            surprise: live.surprise,
        });
    }

    /// Goes back to the list before this one, with its cursor, or from the saved list back to
    /// the results.
    fn back(&mut self) -> Option<Action> {
        if self.view == View::Saved {
            return self.switch_saved();
        }
        let Some(snapshot) = self.history.pop() else {
            self.echo = Some("backspace · no earlier list".into());
            return None;
        };
        let live = self.live.as_mut()?;
        // Drop whatever is still on its way for the list being left.
        live.generation += 1;
        live.loading = false;
        live.replacing = false;
        live.error = None;
        live.query = snapshot.query;
        live.settings = snapshot.settings;
        live.total = snapshot.total;
        live.next_page = snapshot.next_page;
        live.fallback = snapshot.fallback;
        live.surprise = snapshot.surprise;
        self.search.set(&live.settings.text);
        self.results = snapshot.results;
        self.refresh_saved();
        let selected = self.results.selected().cloned();
        self.preview(selected)
    }

    /// Shows the results again if the saved list is showing.
    fn show_results(&mut self) {
        if self.view == View::Saved {
            self.switch_saved();
        }
    }

    /// Swaps the results for the saved list, or back.
    fn switch_saved(&mut self) -> Option<Action> {
        let showing = match self.view {
            View::Results => {
                let mut saved = Results::new(self.saved.entries());
                saved.saved_view = true;
                self.other = std::mem::replace(&mut self.results, saved);
                View::Saved
            }
            View::Saved => {
                self.results = std::mem::take(&mut self.other);
                View::Results
            }
        };
        self.view = showing;
        self.focus = Pane::Results;
        self.refresh_saved();
        let selected = self.results.selected().cloned();
        match selected {
            Some(artwork) => self.preview(Some(artwork)),
            None => Some(Action::ClearImage),
        }
    }

    /// Saves the selected artwork, or unsaves it.
    fn toggle_saved(&mut self) -> Option<Action> {
        let artwork = self.results.selected().cloned()?;
        let row = self.results.selected_row();
        match self.saved.toggle(&artwork) {
            Ok(saved) => {
                let done = if saved { "saved" } else { "removed from saved" };
                self.echo = Some(format!("{done} · {}", artwork.title()));
            }
            Err(error) => {
                self.echo = Some(format!("could not save: {error:#}"));
                return None;
            }
        }
        if self.view == View::Saved {
            // Unsaving takes it off the saved list, and the next one is selected.
            self.results.replace(self.saved.entries());
            let selected = self.results.reselect(Some(artwork.id()), row);
            self.refresh_saved();
            return match selected {
                Some(next) if next.id() != artwork.id() => self.preview(Some(next)),
                Some(_) => None,
                None => Some(Action::ClearImage),
            };
        }
        self.refresh_saved();
        None
    }

    /// Opens the note editor on the selected artwork, if it is saved.
    fn edit_note(&mut self) {
        let Some(artwork) = self.results.selected() else {
            return;
        };
        let id = artwork.id();
        if !self.saved.contains(id) {
            let save = self
                .key_for(Context::Global, &Action::ToggleSaved)
                .unwrap_or_else(|| "m".into());
            self.echo = Some(format!(
                "only saved artworks take notes; save it with {save}"
            ));
            return;
        }
        let title = artwork.title().to_string();
        self.note.open(id, &title, self.saved.note(id));
        self.note.save_keys = self.keys_for(Context::Overlay, &Action::Select);
        self.note.cancel_keys = self.keys_for(Context::Overlay, &Action::Cancel);
        if self.note.cancel_keys.is_empty() {
            self.note.cancel_keys = self.keys_for(Context::Global, &Action::Cancel);
        }
        self.overlay = Some(Overlay::Note);
    }

    /// Saves the note being edited and closes the editor.
    fn save_note(&mut self) {
        self.overlay = None;
        let Some(id) = self.note.id else { return };
        match self.saved.set_note(id, self.note.text()) {
            Ok(()) => self.echo = Some("note saved".into()),
            Err(error) => self.echo = Some(format!("could not save the note: {error:#}")),
        }
        self.show_record(self.results.selected().cloned());
    }

    /// Asks for the next page when the cursor is near the end of a live search's list.
    fn load_more(&mut self) {
        if self.view == View::Saved {
            return;
        }
        let rows_below = self.results.rows_below();
        let (_, loaded) = self.results.counts();
        let Some(live) = &mut self.live else { return };
        let Some(query) = live.query.clone() else {
            return;
        };
        if live.surprise {
            return;
        }
        let reachable = live.total.min(MAX_RESULTS);
        if live.loading || loaded as u64 >= reachable || rows_below > LOAD_AHEAD {
            return;
        }
        live.loading = true;
        let load = Load::Search {
            query,
            page: live.next_page,
        };
        let generation = live.generation;
        self.send(Action::Load { generation, load });
    }

    /// Takes in a list that came back, unless a newer search has started since.
    fn loaded(&mut self, loaded: Loaded) -> Option<Action> {
        let live = self.live.as_mut()?;
        if loaded.generation != live.generation {
            return None;
        }
        live.loading = false;
        live.replacing = false;
        let Listing {
            entries,
            total,
            fallback,
        } = match loaded.result {
            Ok(listing) => listing,
            Err(error) => {
                // Keep what is listed, and say why nothing changed.
                live.error = Some(error);
                return None;
            }
        };
        live.error = None;
        let first = match loaded.load {
            Load::Highlights(_) | Load::Surprise { .. } => true,
            Load::Search { page, .. } => page == 1,
        };
        live.total = total;
        live.fallback = fallback;
        // While the saved list shows, the results keep loading out of sight.
        let hidden = self.view == View::Saved;
        let list = if hidden {
            &mut self.other
        } else {
            &mut self.results
        };
        if first {
            live.next_page = 2;
            let selected = list.replace(entries);
            if hidden {
                return None;
            }
            self.refresh_saved();
            let next = match selected {
                Some(artwork) => Some(Action::ShowArtwork(artwork)),
                None => Some(Action::ClearImage),
            };
            self.load_more();
            next
        } else {
            live.next_page += 1;
            list.append(entries);
            self.refresh_saved();
            self.load_more();
            None
        }
    }

    /// What the status bar says about the list.
    fn counts(&self) -> String {
        let (shown, loaded) = self.results.counts();
        if self.view == View::Saved {
            let noun = if loaded == 1 { "artwork" } else { "artworks" };
            return format!("Saved · {loaded} {noun}");
        }
        let Some(live) = &self.live else {
            return if shown == loaded {
                let noun = if loaded == 1 { "artwork" } else { "artworks" };
                format!("{loaded} {noun}")
            } else {
                format!("{shown} / {loaded} artworks")
            };
        };
        let mut parts = vec![match (&live.query, live.fallback) {
            _ if live.surprise && loaded == 0 && !live.loading => {
                "Surprise · nothing matches".into()
            }
            _ if live.surprise => "Surprise".into(),
            (None, false) => format!("Highlights · {loaded} artworks"),
            (None, true) => format!("Recent public domain · {loaded}"),
            (Some(_), _) => format!("{} / {}", thousands(loaded as u64), thousands(live.total)),
        }];
        if let Some(query) = &live.query {
            parts.extend(query.pivots());
            match query.filter_count() {
                0 => {}
                1 => parts.push("1 filter".into()),
                n => parts.push(format!("{n} filters")),
            }
            if query.sort != Sort::Relevance {
                parts.push(query.sort.label().into());
            }
        }
        if live.query.is_some()
            && !live.surprise
            && live.total > MAX_RESULTS
            && loaded as u64 >= MAX_RESULTS
        {
            parts.push("narrow the search to see more".into());
        }
        if live.rate_limited {
            parts.push("rate limited".into());
        } else if live.loading {
            parts.push("loading…".into());
        }
        if let Some(error) = &live.error {
            parts.push(error.clone());
        }
        parts.join(" · ")
    }

    fn focus_pane(&mut self, pane: Pane) {
        self.focus = pane;
        self.overlay = None;
        self.zen = false;
    }

    /// Moves by `delta` rows in whatever has focus: an overlay, the results or the record.
    fn scroll(&mut self, delta: isize) -> Option<Action> {
        match (self.overlay, self.focus) {
            (Some(Overlay::Help), _) => self.help.scroll(delta),
            (Some(Overlay::Ramps), _) => self.ramps.step(delta),
            (Some(Overlay::Quit), _) => {}
            (Some(Overlay::Record), _) => self.full_record.scroll(delta),
            (Some(Overlay::Filters), _) => self.filters.step(delta),
            (Some(Overlay::Note), _) => {}
            (Some(Overlay::Frames), _) => self.frames.step(delta),
            (None, Pane::Results) => {
                let moved = self.results.step(delta);
                return self.preview(moved);
            }
            (None, Pane::Record) => self.record.scroll(delta),
            (None, Pane::Search | Pane::Art) => {}
        }
        None
    }

    fn half_page(&self) -> isize {
        if self.overlay == Some(Overlay::Record) {
            return self.full_record.half_page();
        }
        match self.focus {
            Pane::Record => self.record.half_page(),
            _ => self.results.half_page(),
        }
    }
}

impl Component for Explorer {
    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.art.register_action_handler(tx.clone())?;
        self.action_tx = Some(tx);
        Ok(())
    }

    fn init(&mut self, _area: ratatui::layout::Size) -> Result<()> {
        if self.live.is_some() {
            self.search_live(String::new());
        } else if let Some(artwork) = self.results.selected() {
            self.send(Action::ShowArtwork(artwork.clone()));
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        let key = normalize(key);
        let context = self.context();
        if self.overlay == Some(Overlay::Filters) && self.filters.is_typing() {
            // A year row takes digits and a leading minus, before any key they are bound to.
            let plain = !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
            match key.code {
                KeyCode::Char(c) if plain && (c.is_ascii_digit() || c == '-') => {
                    self.filters.push(c);
                    return Ok(None);
                }
                KeyCode::Backspace if plain => {
                    self.filters.backspace();
                    return Ok(None);
                }
                _ => {}
            }
        }
        if self.overlay == Some(Overlay::Frames) && self.frames.is_typing() {
            // The custom size takes digits, `x` and spaces, before any key they are bound to.
            let plain = !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
            match key.code {
                KeyCode::Char(c)
                    if plain && (c.is_ascii_digit() || matches!(c, 'x' | 'X' | ' ')) =>
                {
                    self.frames.push(c);
                    return Ok(None);
                }
                KeyCode::Backspace if plain => {
                    self.frames.backspace();
                    return Ok(None);
                }
                _ => {}
            }
        }
        if self.overlay == Some(Overlay::Note) {
            // The note takes every printable key, like the search bar.
            let plain = !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
            match key.code {
                KeyCode::Char(c) if plain => {
                    self.note.push(c);
                    return Ok(None);
                }
                KeyCode::Backspace if plain => {
                    self.note.backspace();
                    return Ok(None);
                }
                _ => {}
            }
        }
        if let Some(action) = self.lookup(context, key).cloned() {
            self.set_echo(key, Some(&action));
            return Ok(Some(action));
        }
        if context == Context::Search {
            // Characters and backspace the search bar does not bind are text. Other keys, such
            // as `tab` and ctrl keys, fall through to `Global`.
            let plain = !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
            match key.code {
                KeyCode::Char(c) if plain => {
                    self.search.push(c);
                    return Ok(None);
                }
                KeyCode::Backspace if plain => {
                    self.search.backspace();
                    return Ok(None);
                }
                _ => {}
            }
        }
        let action = self.lookup(Context::Global, key).cloned();
        self.set_echo(key, action.as_ref());
        Ok(action)
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<Option<Action>> {
        let at = Position::new(mouse.column, mouse.row);
        match mouse.kind {
            MouseEventKind::Drag(MouseButton::Left) => {
                // Dragging the art moves the image with the pointer.
                let Some(from) = self.drag.replace(at) else {
                    return Ok(None);
                };
                let (cols, rows) = (
                    i32::from(from.x) - i32::from(at.x),
                    i32::from(from.y) - i32::from(at.y),
                );
                return Ok(((cols, rows) != (0, 0)).then_some(Action::PanBy { cols, rows }));
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag = None;
                return Ok(None);
            }
            _ => {}
        }
        let delta = match mouse.kind {
            MouseEventKind::ScrollDown => 1,
            MouseEventKind::ScrollUp => -1,
            MouseEventKind::Down(MouseButton::Left) => 0,
            _ => return Ok(None),
        };
        if delta == 0 {
            // A press starts a drag only over the art, below.
            self.drag = None;
        }
        if let Some(overlay) = self.overlay {
            let area = match overlay {
                Overlay::Help => self.help.area,
                Overlay::Ramps => self.ramps.area,
                Overlay::Quit => self.quit.area,
                Overlay::Record => self.full_record.area,
                Overlay::Filters => self.filters.area,
                Overlay::Note => self.note.area,
                Overlay::Frames => self.frames.area,
            };
            if delta == 0 && !area.contains(at) {
                self.overlay = None;
            } else if delta != 0 {
                return Ok(self.scroll(delta));
            }
            return Ok(None);
        }
        let pane = if self.zen {
            Some(Pane::Art)
        } else {
            self.areas.pane_at(at)
        };
        let Some(pane) = pane else {
            return Ok(None);
        };
        if pane == Pane::Art {
            if delta == 0 {
                self.drag = Some(at);
                if !self.zen {
                    self.focus = pane;
                }
                return Ok(None);
            }
            // Scrolling up zooms in, about the cell under the pointer.
            return Ok(Some(Action::ZoomAt {
                column: at.x,
                row: at.y,
                steps: if delta < 0 { 1 } else { -1 },
            }));
        }
        Ok(match (pane, delta) {
            (Pane::Results, 0) => {
                self.focus = pane;
                let clicked = self.results.click(at.x, at.y);
                self.preview(clicked)
            }
            (Pane::Results, _) => {
                let moved = self.results.step(delta);
                self.preview(moved)
            }
            (Pane::Record, 0) | (Pane::Search, 0) | (Pane::Art, _) => {
                self.focus = pane;
                None
            }
            (Pane::Record, _) => {
                self.record.scroll(delta);
                None
            }
            (Pane::Search, _) => None,
        })
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let next = match &action {
            Action::ToggleHelp => {
                if self.overlay == Some(Overlay::Help) {
                    self.overlay = None;
                } else {
                    self.help.open(&self.keybindings, self.pane_context());
                    self.overlay = Some(Overlay::Help);
                }
                None
            }
            Action::PickRamp => {
                self.overlay = Some(Overlay::Ramps);
                None
            }
            Action::PickFrame => {
                self.frames.open(self.art.frame(), self.screen);
                self.overlay = Some(Overlay::Frames);
                None
            }
            Action::Select if self.overlay == Some(Overlay::Frames) => {
                let chosen = self.frames.choose();
                if chosen.is_some() {
                    self.overlay = None;
                }
                chosen
            }
            Action::Cancel => {
                if self.overlay.take().is_none() {
                    if self.zen {
                        self.zen = false;
                    } else if self.focus == Pane::Search {
                        self.focus = Pane::Results;
                    }
                }
                None
            }
            Action::FocusSearch => {
                self.focus_pane(Pane::Search);
                None
            }
            Action::FocusResults => {
                self.focus_pane(Pane::Results);
                None
            }
            Action::FocusArt => {
                self.focus_pane(Pane::Art);
                None
            }
            Action::FocusRecord => {
                self.focus_pane(Pane::Record);
                None
            }
            Action::FocusNext => {
                self.focus_pane(self.focus.step(1));
                None
            }
            Action::FocusPrevious => {
                self.focus_pane(self.focus.step(-1));
                None
            }
            Action::ToggleZen => {
                self.zen = !self.zen;
                None
            }
            Action::NextArtwork => {
                let moved = self.results.step(1);
                self.preview(moved)
            }
            Action::PreviousArtwork => {
                let moved = self.results.step(-1);
                self.preview(moved)
            }
            Action::MoveDown => self.scroll(1),
            Action::MoveUp => self.scroll(-1),
            Action::Top => self.scroll(isize::MIN / 2),
            Action::Bottom => self.scroll(isize::MAX / 2),
            Action::HalfPageDown => self.scroll(self.half_page()),
            Action::HalfPageUp => self.scroll(-self.half_page()),
            Action::Select if self.overlay == Some(Overlay::Ramps) => self.ramps.choose(),
            Action::Select if self.overlay == Some(Overlay::Quit) => Some(Action::Quit),
            Action::AskQuit if self.overlay == Some(Overlay::Quit) => Some(Action::Quit),
            Action::AskQuit => {
                self.ask_quit();
                None
            }
            Action::RunSearch => {
                let query = self.search.submit();
                self.focus = Pane::Results;
                if self.view == View::Saved {
                    self.switch_saved();
                }
                if self.live.is_some() {
                    self.search_live(query);
                    None
                } else {
                    let moved = self.results.filter(&query);
                    if self.results.selected().is_none() {
                        self.show_record(None);
                    }
                    self.preview(moved)
                }
            }
            Action::Loaded(loaded) => self.loaded(loaded.clone()),
            Action::ToggleFullRecord => self.record.toggle_expanded().map(Action::FetchRecord),
            Action::OpenRecord => {
                self.full_record.show(self.results.selected().cloned());
                self.overlay = Some(Overlay::Record);
                self.full_record.wants_full().map(Action::FetchRecord)
            }
            Action::RecordLoaded { id, result } => {
                self.record.loaded(*id, result);
                self.full_record.loaded(*id, result);
                None
            }
            Action::OpenFilters => {
                if let Some(live) = &self.live {
                    self.filters.open(&live.settings.filters);
                    self.overlay = Some(Overlay::Filters);
                    if !self.filters.has_facets() && !self.facets_requested {
                        self.facets_requested = true;
                        self.send(Action::FetchFacets);
                    }
                } else {
                    self.echo = Some("f · filters need a live search, not --from-file".into());
                }
                None
            }
            Action::FacetsLoaded(facets) => {
                self.filters.set_facets(facets.clone());
                if facets.is_err() {
                    // Try again the next time the filters open.
                    self.facets_requested = false;
                }
                None
            }
            Action::PreviousOption if self.overlay == Some(Overlay::Filters) => {
                self.filters.change(-1);
                None
            }
            Action::NextOption if self.overlay == Some(Overlay::Filters) => {
                self.filters.change(1);
                None
            }
            Action::Select if self.overlay == Some(Overlay::Note) => {
                self.save_note();
                None
            }
            Action::ClearSearch if self.overlay == Some(Overlay::Note) => {
                self.note.clear();
                None
            }
            Action::ClearSearch if self.overlay == Some(Overlay::Filters) => {
                self.filters.reset(&default_filters());
                None
            }
            Action::Select if self.overlay == Some(Overlay::Filters) => {
                match self.filters.apply() {
                    Ok(filters) => {
                        self.overlay = None;
                        self.focus = Pane::Results;
                        let settings = self.live.as_ref().map(|live| {
                            let mut settings = live.settings.clone();
                            if filters.artist_id != settings.filters.artist_id {
                                settings.artist = None;
                            }
                            if filters.department_id != settings.filters.department_id {
                                settings.department = None;
                            }
                            settings.filters = filters;
                            settings
                        });
                        if let Some(settings) = settings {
                            self.run_live(settings);
                        }
                    }
                    Err(error) => self.filters.fail(error),
                }
                None
            }
            Action::CycleSort => {
                let settings = self.live.as_ref().map(|live| Query {
                    sort: live.settings.sort.next(),
                    ..live.settings.clone()
                });
                if let Some(settings) = settings {
                    self.run_live(settings);
                } else {
                    self.echo = Some("s · sorting needs a live search, not --from-file".into());
                }
                None
            }
            Action::MoreByArtist => {
                self.pivot(true);
                None
            }
            Action::MoreFromDepartment => {
                self.pivot(false);
                None
            }
            Action::Surprise => self.surprise(),
            Action::Back if self.overlay.is_none() => self.back(),
            Action::ToggleSaved => self.toggle_saved(),
            Action::SwitchSaved => self.switch_saved(),
            Action::EditNote => {
                self.edit_note();
                None
            }
            Action::OpenInBrowser => self.link().map(Action::OpenUrl),
            Action::CopyLink => self.link().map(Action::CopyText),
            Action::RateLimited(limited) => {
                if let Some(live) = &mut self.live {
                    live.rate_limited = *limited;
                }
                None
            }
            Action::EarlierQuery => {
                self.search.earlier();
                None
            }
            Action::LaterQuery => {
                self.search.later();
                None
            }
            Action::ClearSearch => {
                self.search.clear();
                None
            }
            _ => None,
        };
        // The log records every action, and the HUD collects timings.
        self.art.update(action.clone())?;
        self.hud.update(action.clone())?;
        self.logs.update(action)?;
        Ok(next)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        let [search, middle, status] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);
        let [results, art, record] = Layout::horizontal([
            Constraint::Ratio(1, 4),
            Constraint::Fill(1),
            Constraint::Ratio(1, 4),
        ])
        .areas(middle);
        // The Pane and Screen frames mean the same sizes in zen and out of it.
        let pane = pane_block(3, "Art", false).inner(art);
        self.screen = (area.width, area.height);
        self.art.set_sizes((pane.width, pane.height), self.screen);
        if self.zen {
            self.areas = Areas::default();
            self.art.draw(frame, area)?;
            self.hud.draw(frame, area)?;
            self.logs.draw(frame, area)?;
        } else {
            self.areas = Areas {
                search,
                results,
                art,
                record,
            };

            let focused = |pane| self.overlay.is_none() && self.focus == pane;
            self.search.focused = focused(Pane::Search);
            self.results.focused = focused(Pane::Results);
            self.record.focused = focused(Pane::Record);
            self.search.draw(frame, search)?;
            self.results.draw(frame, results)?;
            self.record.draw(frame, record)?;
            let block = pane_block(3, "Art", focused(Pane::Art));
            let inner = block.inner(art);
            frame.render_widget(block, art);
            self.art.draw(frame, inner)?;
            self.hud.draw(frame, inner)?;
            self.logs.draw(frame, inner)?;

            self.status.counts = self.counts();
            if let Some(view) = self.art.describe() {
                self.status.counts.push_str(" · ");
                self.status.counts.push_str(&view);
                if self.art.clipped() {
                    self.status.counts.push_str(" (clipped; z shows all)");
                }
            }
            self.status.hints = self.hints();
            self.status.echo = self.echo.clone();
            self.status.draw(frame, status)?;
        }
        match self.overlay {
            Some(Overlay::Help) => self.help.draw(frame, area)?,
            Some(Overlay::Ramps) => self.ramps.draw(frame, area)?,
            Some(Overlay::Quit) => self.quit.draw(frame, area)?,
            Some(Overlay::Record) => self.full_record.draw(frame, area)?,
            Some(Overlay::Filters) => self.filters.draw(frame, area)?,
            Some(Overlay::Note) => self.note.draw(frame, area)?,
            Some(Overlay::Frames) => self.frames.draw(frame, area)?,
            None => {}
        }
        Ok(())
    }
}

/// A seed for the surprises from the clock, never zero.
fn clock_seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or_default();
    nanos | 1
}

/// `n` with commas between thousands, such as `10,000`.
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use charaoke::ColorDepth;

    use super::*;
    use crate::components::{
        art_view::FrameSize,
        test_support::{ARTWORKS_FIXTURE, render},
    };

    fn explorer() -> Explorer {
        let config = Config::new().expect("default config");
        let artworks = serde_json::from_str(ARTWORKS_FIXTURE).expect("fixture parses");
        Explorer::with_art(
            Source::File(Entry::from_file(artworks)),
            &config,
            Home::with_depth(ColorDepth::TrueColor),
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn char(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    /// Presses `key` and applies what it maps to, returning what that asks for next.
    fn press(explorer: &mut Explorer, key: KeyEvent) -> Option<Action> {
        let action = explorer.handle_key_event(key).unwrap()?;
        explorer.update(action).unwrap()
    }

    fn selected_title(explorer: &Explorer) -> Option<String> {
        explorer.selected().map(|a| a.title().to_string())
    }

    #[test]
    fn renders_the_explorer() {
        let mut explorer = explorer();
        insta::assert_snapshot!(render(&mut explorer, 100, 16));
    }

    #[test]
    fn renders_the_help_overlay() {
        let mut explorer = explorer();
        press(&mut explorer, char('?'));
        insta::assert_snapshot!(render(&mut explorer, 100, 40));
    }

    #[test]
    fn zen_draws_only_the_art() {
        let mut explorer = explorer();
        press(&mut explorer, char('z'));
        assert!(explorer.zen());
        assert!(!render(&mut explorer, 60, 10).contains("Results"));
        press(&mut explorer, key(KeyCode::Esc));
        assert!(!explorer.zen());
    }

    #[test]
    fn n_and_p_move_the_selection_from_any_pane() {
        let mut explorer = explorer();
        press(&mut explorer, char('3'));
        assert_eq!(explorer.focus(), Pane::Art);
        let next = press(&mut explorer, char('n'));
        assert!(matches!(next, Some(Action::ShowArtwork(a)) if a.title() == "The Bedroom"));
        press(&mut explorer, char('p'));
        assert_eq!(
            selected_title(&explorer).as_deref(),
            Some("A Sunday on La Grande Jatte — 1884")
        );
        assert_eq!(press(&mut explorer, char('p')), None, "already at the top");
    }

    #[test]
    fn j_moves_the_list_or_scrolls_the_record_by_focus() {
        let mut explorer = explorer();
        render(&mut explorer, 100, 16);
        assert!(press(&mut explorer, char('j')).is_some(), "the list moved");
        press(&mut explorer, char('4'));
        assert_eq!(press(&mut explorer, char('j')), None, "the record scrolled");
        assert_eq!(selected_title(&explorer).as_deref(), Some("The Bedroom"));
    }

    #[test]
    fn the_search_bar_takes_text_and_filters_on_enter() {
        let mut explorer = explorer();
        press(&mut explorer, char('/'));
        assert_eq!(explorer.focus(), Pane::Search);
        for c in "q?wave".chars() {
            assert_eq!(press(&mut explorer, char(c)), None);
        }
        assert_eq!(explorer.search.query(), "q?wave", "q and ? type themselves");
        assert_eq!(explorer.overlay(), None);
        press(&mut explorer, key(KeyCode::Backspace));
        for _ in 0..4 {
            press(&mut explorer, key(KeyCode::Backspace));
        }
        assert_eq!(explorer.search.query(), "q");
        explorer.search.clear();
        "WAVE".chars().for_each(|c| {
            press(&mut explorer, char(c));
        });
        let next = press(&mut explorer, key(KeyCode::Enter));
        assert!(matches!(next, Some(Action::ShowArtwork(a)) if a.id() == 24645));
        assert_eq!(explorer.focus(), Pane::Results);
        assert_eq!(explorer.results.counts(), (1, 5));
    }

    #[test]
    fn d_expands_the_record_and_asks_for_the_full_record() {
        let mut explorer = explorer();
        press(&mut explorer, char('4'));
        assert_eq!(
            press(&mut explorer, char('d')),
            Some(Action::FetchRecord(27992))
        );
        assert_eq!(press(&mut explorer, char('d')), None, "collapsing");
    }

    #[test]
    fn enter_in_the_record_opens_the_full_record_and_follows_the_selection() {
        let mut explorer = explorer();
        press(&mut explorer, char('4'));
        assert_eq!(
            press(&mut explorer, key(KeyCode::Enter)),
            Some(Action::FetchRecord(27992))
        );
        assert_eq!(explorer.overlay(), Some(Overlay::Record));
        let full = acres::artworks::Artwork {
            summary: explorer.selected().unwrap().summary.clone(),
            dimensions: Some("207.5 × 308.1 cm".into()),
            ..Default::default()
        };
        explorer
            .update(Action::RecordLoaded {
                id: 27992,
                result: Ok(full),
            })
            .unwrap();
        insta::assert_snapshot!(render(&mut explorer, 100, 24));
        // `n` moves on under the overlay, which fetches the next record.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        explorer.action_tx = Some(tx);
        press(&mut explorer, char('n'));
        assert_eq!(rx.try_recv().ok(), Some(Action::FetchRecord(28560)));
        press(&mut explorer, key(KeyCode::Esc));
        assert_eq!(explorer.overlay(), None);
    }

    #[test]
    fn o_and_y_hand_the_artwork_link_to_the_app() {
        let mut explorer = explorer();
        let link = "https://www.artic.edu/artworks/27992".to_string();
        assert_eq!(
            press(&mut explorer, char('o')),
            Some(Action::OpenUrl(link.clone()))
        );
        assert_eq!(
            press(&mut explorer, char('y')),
            Some(Action::CopyText(link))
        );
    }

    #[test]
    fn filters_and_sort_say_they_need_a_live_search() {
        let mut explorer = explorer();
        assert_eq!(press(&mut explorer, char('f')), None);
        assert_eq!(explorer.overlay(), None);
        assert!(explorer.echo.as_deref().unwrap().contains("live search"));
    }

    #[test]
    fn q_asks_before_quitting() {
        let mut explorer = explorer();
        assert_eq!(press(&mut explorer, char('q')), None);
        assert_eq!(explorer.overlay(), Some(Overlay::Quit));
        insta::assert_snapshot!(render(&mut explorer, 100, 16));
        assert_eq!(press(&mut explorer, char('q')), Some(Action::Quit));

        let mut explorer = self::explorer();
        press(&mut explorer, char('q'));
        assert_eq!(
            press(&mut explorer, key(KeyCode::Enter)),
            Some(Action::Quit)
        );

        let mut explorer = self::explorer();
        press(&mut explorer, char('q'));
        assert_eq!(press(&mut explorer, key(KeyCode::Esc)), None);
        assert_eq!(explorer.overlay(), None, "esc stays");
    }

    #[test]
    fn enter_and_space_ask_for_the_full_image() {
        let mut explorer = explorer();
        let action = |explorer: &mut Explorer, key| explorer.handle_key_event(key).unwrap();
        assert_eq!(
            action(&mut explorer, key(KeyCode::Enter)),
            Some(Action::ShowFullImage)
        );
        assert_eq!(
            action(&mut explorer, char(' ')),
            Some(Action::ShowFullImage)
        );
        press(&mut explorer, char('3'));
        assert_eq!(
            action(&mut explorer, key(KeyCode::Enter)),
            Some(Action::ShowFullImage)
        );
        press(&mut explorer, char('/'));
        assert_eq!(
            action(&mut explorer, char(' ')),
            None,
            "a space in the query"
        );
    }

    #[test]
    fn ctrl_keys_still_reach_global_from_the_search_bar() {
        let mut explorer = explorer();
        press(&mut explorer, char('/'));
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(
            explorer.handle_key_event(ctrl_c).unwrap(),
            Some(Action::Quit)
        );
    }

    #[test]
    fn esc_closes_overlays_then_leaves_the_search_bar() {
        let mut explorer = explorer();
        press(&mut explorer, char('/'));
        press(&mut explorer, key(KeyCode::Esc));
        assert_eq!(explorer.focus(), Pane::Results);
        press(&mut explorer, char('r'));
        assert_eq!(explorer.overlay(), Some(Overlay::Ramps));
        press(&mut explorer, key(KeyCode::Esc));
        assert_eq!(explorer.overlay(), None);
    }

    #[test]
    fn ramps_take_j_and_enter_while_open() {
        let mut explorer = explorer();
        press(&mut explorer, char('r'));
        press(&mut explorer, char('j'));
        let chosen = press(&mut explorer, key(KeyCode::Enter));
        assert!(matches!(chosen, Some(Action::SetRamp(_))));
        assert_eq!(
            selected_title(&explorer).as_deref(),
            Some("A Sunday on La Grande Jatte — 1884")
        );
    }

    #[test]
    fn tab_cycles_the_panes_both_ways() {
        let mut explorer = explorer();
        press(&mut explorer, key(KeyCode::Tab));
        assert_eq!(explorer.focus(), Pane::Art);
        let shift_tab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        press(&mut explorer, shift_tab);
        press(&mut explorer, shift_tab);
        assert_eq!(explorer.focus(), Pane::Search);
        press(&mut explorer, shift_tab);
        assert_eq!(explorer.focus(), Pane::Record, "wraps around");
    }

    #[test]
    fn the_status_bar_echoes_the_last_key() {
        let mut explorer = explorer();
        press(&mut explorer, char('n'));
        assert_eq!(explorer.echo.as_deref(), Some("n · next artwork"));
        press(&mut explorer, char('x'));
        assert_eq!(explorer.echo.as_deref(), Some("x · not mapped here"));
        let text = render(&mut explorer, 100, 16);
        assert!(
            text.lines()
                .last()
                .unwrap()
                .ends_with("x · not mapped here")
        );
    }

    #[test]
    fn show_keys_off_keeps_the_status_bar_quiet() {
        let mut explorer = explorer();
        explorer.show_keys = false;
        press(&mut explorer, char('n'));
        assert_eq!(explorer.echo, None);
    }

    #[test]
    fn shifted_keys_match_whether_or_not_shift_is_reported() {
        let mut explorer = explorer();
        let with_shift = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
        assert_eq!(
            explorer.handle_key_event(with_shift).unwrap(),
            Some(Action::ToggleHelp)
        );
    }

    #[test]
    fn help_lists_the_pane_that_had_focus() {
        let mut explorer = explorer();
        press(&mut explorer, char('4'));
        press(&mut explorer, char('?'));
        let text = render(&mut explorer, 100, 40);
        assert!(text.contains("Record"));
        assert!(text.contains("Anywhere"));
        press(&mut explorer, char('?'));
        assert_eq!(explorer.overlay(), None, "? closes it again");
    }

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn clicks_focus_panes_and_select_rows() {
        let mut explorer = explorer();
        render(&mut explorer, 100, 16);
        let click = MouseEventKind::Down(MouseButton::Left);
        // The record pane starts three quarters of the way across.
        explorer.handle_mouse_event(mouse(click, 90, 8)).unwrap();
        assert_eq!(explorer.focus(), Pane::Record);
        // Results rows start below the search bar and the list's own border.
        let picked = explorer.handle_mouse_event(mouse(click, 5, 6)).unwrap();
        assert_eq!(explorer.focus(), Pane::Results);
        assert!(matches!(picked, Some(Action::ShowArtwork(a)) if a.title() == "Water Lilies"));
    }

    #[test]
    fn scrolling_over_the_list_moves_the_selection() {
        let mut explorer = explorer();
        render(&mut explorer, 100, 16);
        let moved = explorer
            .handle_mouse_event(mouse(MouseEventKind::ScrollDown, 5, 6))
            .unwrap();
        assert!(moved.is_some());
        assert_eq!(selected_title(&explorer).as_deref(), Some("The Bedroom"));
        let art = explorer
            .handle_mouse_event(mouse(MouseEventKind::ScrollDown, 50, 8))
            .unwrap();
        assert_eq!(
            art,
            Some(Action::ZoomAt {
                column: 50,
                row: 8,
                steps: -1
            }),
            "scrolling down over the art zooms out"
        );
    }

    #[test]
    fn dragging_the_art_pans_it_and_dragging_elsewhere_does_not() {
        let mut explorer = explorer();
        render(&mut explorer, 100, 16);
        let press = MouseEventKind::Down(MouseButton::Left);
        let drag = MouseEventKind::Drag(MouseButton::Left);
        explorer.handle_mouse_event(mouse(press, 50, 8)).unwrap();
        assert_eq!(explorer.focus(), Pane::Art);
        let panned = explorer.handle_mouse_event(mouse(drag, 47, 9)).unwrap();
        assert_eq!(panned, Some(Action::PanBy { cols: 3, rows: -1 }));
        explorer
            .handle_mouse_event(mouse(MouseEventKind::Up(MouseButton::Left), 47, 9))
            .unwrap();
        explorer.handle_mouse_event(mouse(press, 5, 6)).unwrap();
        assert_eq!(
            explorer.handle_mouse_event(mouse(drag, 8, 7)).unwrap(),
            None
        );
    }

    #[test]
    fn zen_takes_the_art_keys() {
        let mut explorer = explorer();
        let bound = |explorer: &mut Explorer, c| explorer.handle_key_event(char(c)).unwrap();
        press(&mut explorer, char('z'));
        assert_eq!(bound(&mut explorer, 'j'), Some(Action::PanDown));
        assert_eq!(bound(&mut explorer, '+'), Some(Action::ZoomIn));
        assert_eq!(bound(&mut explorer, 'n'), Some(Action::NextArtwork));
        press(&mut explorer, key(KeyCode::Esc));
        assert_eq!(bound(&mut explorer, 'j'), Some(Action::MoveDown));
    }

    #[test]
    fn s_picks_a_frame_and_the_status_bar_names_it() {
        let mut explorer = explorer();
        explorer.update(Action::FocusArt).unwrap();
        render(&mut explorer, 100, 16);
        let pick = explorer.handle_key_event(char('S')).unwrap();
        assert_eq!(pick, Some(Action::PickFrame));
        explorer.update(Action::PickFrame).unwrap();
        assert_eq!(explorer.overlay(), Some(Overlay::Frames));
        explorer.update(Action::MoveDown).unwrap();
        explorer.update(Action::MoveDown).unwrap();
        let chosen = explorer.update(Action::Select).unwrap();
        assert_eq!(
            chosen,
            Some(Action::SetFrame(FrameSize::Custom { cols: 20, rows: 12 }))
        );
        assert_eq!(explorer.overlay(), None);
        explorer.update(chosen.unwrap()).unwrap();
        assert!(render(&mut explorer, 100, 16).contains("stamp 20x12"));
    }

    #[test]
    fn a_custom_frame_types_its_size_in_the_picker() {
        let mut explorer = explorer();
        render(&mut explorer, 100, 16);
        explorer.update(Action::PickFrame).unwrap();
        explorer.update(Action::Bottom).unwrap();
        for c in "30x9".chars() {
            assert_eq!(press(&mut explorer, char(c)), None);
        }
        assert_eq!(
            explorer.update(Action::Select).unwrap(),
            Some(Action::SetFrame(FrameSize::Custom { cols: 30, rows: 9 }))
        );
    }

    #[test]
    fn a_click_outside_an_overlay_closes_it() {
        let mut explorer = explorer();
        press(&mut explorer, char('?'));
        render(&mut explorer, 100, 40);
        let click = MouseEventKind::Down(MouseButton::Left);
        explorer.handle_mouse_event(mouse(click, 50, 20)).unwrap();
        assert_eq!(explorer.overlay(), Some(Overlay::Help), "inside");
        explorer.handle_mouse_event(mouse(click, 0, 0)).unwrap();
        assert_eq!(explorer.overlay(), None);
    }

    #[test]
    fn m_saves_and_the_saved_list_takes_notes() {
        let mut explorer = explorer();
        press(&mut explorer, char('n'));
        press(&mut explorer, char('m'));
        assert_eq!(explorer.echo.as_deref(), Some("saved · The Bedroom"));
        assert!(render(&mut explorer, 100, 16).contains("* The Bedroom"));

        let shown = press(&mut explorer, char('\''));
        assert_eq!(explorer.view(), View::Saved);
        assert!(matches!(shown, Some(Action::ShowArtwork(e)) if e.title() == "The Bedroom"));
        press(&mut explorer, char('e'));
        assert_eq!(explorer.overlay(), Some(Overlay::Note));
        // Keys bound elsewhere, such as `q`, type into the note.
        "quiet".chars().for_each(|c| {
            press(&mut explorer, char(c));
        });
        press(&mut explorer, key(KeyCode::Backspace));
        press(&mut explorer, key(KeyCode::Enter));
        assert_eq!(explorer.overlay(), None);
        assert_eq!(explorer.saved.note(28560), Some("quie"));
        insta::assert_snapshot!(render(&mut explorer, 100, 20));

        // Unsaving from the saved list takes it off.
        assert_eq!(press(&mut explorer, char('m')), Some(Action::ClearImage));
        assert_eq!(explorer.selected(), None);
        assert_eq!(explorer.counts(), "Saved · 0 artworks");
        press(&mut explorer, key(KeyCode::Backspace));
        assert_eq!(explorer.view(), View::Results);
        assert_eq!(selected_title(&explorer).as_deref(), Some("The Bedroom"));
        assert!(!render(&mut explorer, 100, 16).contains("* The Bedroom"));
    }

    #[test]
    fn e_on_an_unsaved_artwork_says_how_to_save_it() {
        let mut explorer = explorer();
        press(&mut explorer, char('e'));
        assert_eq!(explorer.overlay(), None);
        assert_eq!(
            explorer.echo.as_deref(),
            Some("only saved artworks take notes; save it with m")
        );
    }

    #[test]
    fn esc_leaves_a_note_unchanged() {
        let mut explorer = explorer();
        press(&mut explorer, char('m'));
        press(&mut explorer, char('e'));
        press(&mut explorer, char('x'));
        press(&mut explorer, key(KeyCode::Esc));
        assert_eq!(explorer.overlay(), None);
        assert_eq!(explorer.saved.note(27992), None);
    }

    #[test]
    fn file_mode_pivots_need_a_live_search_and_r_picks_a_row() {
        let mut explorer = explorer();
        press(&mut explorer, char('A'));
        assert_eq!(
            explorer.echo.as_deref(),
            Some("more by artist needs a live search, not --from-file")
        );
        explorer.seed = 7;
        let picks: std::collections::HashSet<u32> = (0..20)
            .filter_map(|_| {
                press(&mut explorer, char('R'));
                explorer.selected().map(Entry::id)
            })
            .collect();
        assert!(picks.len() > 1, "surprises move around the list: {picks:?}");
    }

    #[test]
    fn from_file_reads_a_saved_list_or_an_artworks_file() {
        let mut list = SavedList::in_memory();
        let artworks = Entry::from_text(ARTWORKS_FIXTURE).unwrap();
        assert_eq!(artworks.len(), 5);
        list.toggle(&artworks[1]).unwrap();
        let path = std::env::temp_dir().join(format!("acres-tui-from-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        let mut on_disk = SavedList::open(path.join("saved.json")).unwrap();
        on_disk.toggle(&artworks[1]).unwrap();
        let text = std::fs::read_to_string(path.join("saved.json")).unwrap();
        assert_eq!(Entry::from_text(&text).unwrap(), list.entries());
    }

    mod live {
        use tokio::sync::mpsc::{self, UnboundedReceiver};

        use super::*;
        use crate::components::catalog::PAGE_SIZE;

        fn entry(id: u32) -> Entry {
            Entry {
                summary: acres::artworks::ArtworkSummary {
                    id,
                    title: format!("Artwork {id}"),
                    image_id: Some(format!("image-{id}")),
                    artist_id: Some(35),
                    artist_title: Some("Claude Monet".into()),
                    department_id: Some("PC-3".into()),
                    department_title: Some("Painting and Sculpture of Europe".into()),
                    ..Default::default()
                },
                config: acres::ApiConfig {
                    iiif_url: "https://iiif.example/iiif/2".into(),
                    ..Default::default()
                },
            }
        }

        fn listing(ids: std::ops::Range<u32>, total: u64) -> Result<Listing, String> {
            Ok(Listing {
                entries: ids.map(entry).collect(),
                total,
                fallback: false,
            })
        }

        /// A live explorer, started, with what it has asked for so far.
        fn started() -> (Explorer, UnboundedReceiver<Action>) {
            let config = Config::new().expect("default config");
            let source = Source::Live {
                highlights: vec![24645, 16568],
            };
            let mut explorer =
                Explorer::with_art(source, &config, Home::with_depth(ColorDepth::TrueColor));
            let (tx, rx) = mpsc::unbounded_channel();
            explorer.register_action_handler(tx).unwrap();
            explorer.init(ratatui::layout::Size::new(100, 30)).unwrap();
            (explorer, rx)
        }

        fn loads(rx: &mut UnboundedReceiver<Action>) -> Vec<(u64, Load)> {
            let mut loads = Vec::new();
            while let Ok(action) = rx.try_recv() {
                if let Action::Load { generation, load } = action {
                    loads.push((generation, load));
                }
            }
            loads
        }

        fn deliver(
            explorer: &mut Explorer,
            generation: u64,
            load: Load,
            result: Result<Listing, String>,
        ) -> Option<Action> {
            explorer
                .update(Action::Loaded(Loaded {
                    generation,
                    load,
                    result,
                }))
                .unwrap()
        }

        fn search(explorer: &mut Explorer, query: &str) {
            press(explorer, char('/'));
            explorer.search.clear();
            query.chars().for_each(|c| {
                press(explorer, char(c));
            });
            press(explorer, key(KeyCode::Enter));
        }

        /// The query of the one search asked for since the last check.
        fn searched(rx: &mut UnboundedReceiver<Action>) -> Query {
            match loads(rx).as_slice() {
                [(_, Load::Search { query, page: 1 })] => query.clone(),
                other => panic!("expected one search, got {other:?}"),
            }
        }

        #[test]
        fn filters_apply_on_enter_and_rerun_the_search() {
            let (mut explorer, mut rx) = started();
            search(&mut explorer, "monet");
            loads(&mut rx);
            press(&mut explorer, char('f'));
            assert_eq!(explorer.overlay(), Some(Overlay::Filters));
            assert!(
                std::iter::from_fn(|| rx.try_recv().ok()).any(|a| a == Action::FetchFacets),
                "the departments and types are asked for"
            );
            // Public domain: yes, then from 1850, typed even though digits focus panes.
            press(&mut explorer, char('j'));
            press(&mut explorer, char('l'));
            for _ in 0..4 {
                press(&mut explorer, char('j'));
            }
            for c in "1850".chars() {
                press(&mut explorer, char(c));
            }
            assert_eq!(explorer.overlay(), Some(Overlay::Filters), "still open");
            press(&mut explorer, key(KeyCode::Enter));
            assert_eq!(explorer.overlay(), None);
            let query = searched(&mut rx);
            assert_eq!(query.text, "monet");
            assert_eq!(query.filters.is_public_domain, Some(true));
            assert_eq!(query.filters.years, Some((1850, 2100)));
            assert!(query.filters.has_image);
            explorer.live.as_mut().unwrap().total = 120;
            explorer.results.replace(vec![entry(1)]);
            assert!(explorer.counts().contains("2 filters"));
        }

        #[test]
        fn esc_drops_filter_edits() {
            let (mut explorer, mut rx) = started();
            loads(&mut rx);
            press(&mut explorer, char('f'));
            press(&mut explorer, char('l'));
            press(&mut explorer, key(KeyCode::Esc));
            assert!(loads(&mut rx).is_empty(), "nothing searched");
            press(&mut explorer, char('f'));
            assert_eq!(explorer.filters.apply().unwrap(), default_filters());
        }

        #[test]
        fn filters_without_text_browse_the_collection_and_reset_goes_back_to_highlights() {
            let (mut explorer, mut rx) = started();
            loads(&mut rx);
            press(&mut explorer, char('f'));
            press(&mut explorer, char('j'));
            press(&mut explorer, char('j'));
            press(&mut explorer, char('l'));
            press(&mut explorer, key(KeyCode::Enter));
            let query = searched(&mut rx);
            assert_eq!(query.text, "");
            assert_eq!(query.filters.is_on_view, Some(true));
            press(&mut explorer, char('f'));
            let ctrl_u = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
            press(&mut explorer, ctrl_u);
            press(&mut explorer, key(KeyCode::Enter));
            assert!(matches!(
                loads(&mut rx).as_slice(),
                [(_, Load::Highlights(_))]
            ));
        }

        #[test]
        fn s_cycles_the_sort_and_reruns_the_search() {
            let (mut explorer, mut rx) = started();
            search(&mut explorer, "monet");
            loads(&mut rx);
            press(&mut explorer, char('s'));
            assert_eq!(searched(&mut rx).sort, Sort::Oldest);
            press(&mut explorer, char('s'));
            let query = searched(&mut rx);
            assert_eq!(query.sort, Sort::Newest);
            explorer.results.replace(vec![entry(1)]);
            assert!(explorer.counts().contains("newest first"));
            press(&mut explorer, char('s'));
            assert_eq!(searched(&mut rx).sort, Sort::Title);
            press(&mut explorer, char('s'));
            assert_eq!(searched(&mut rx).sort, Sort::Relevance);
        }

        #[test]
        fn starts_on_the_highlights() {
            let (mut explorer, mut rx) = started();
            assert_eq!(loads(&mut rx), [(1, Load::Highlights(vec![24645, 16568]))]);
            assert!(explorer.counts().contains("loading"));
            let shown = deliver(&mut explorer, 1, Load::Highlights(vec![]), listing(1..3, 2));
            assert!(matches!(shown, Some(Action::ShowArtwork(a)) if a.id() == 1));
            assert_eq!(explorer.counts(), "Highlights · 2 artworks");
            assert!(loads(&mut rx).is_empty(), "the Highlights are not paged");
        }

        #[test]
        fn a_list_for_an_older_search_is_dropped() {
            let (mut explorer, mut rx) = started();
            search(&mut explorer, "monet");
            let monet = Load::Search {
                query: Query::text("monet"),
                page: 1,
            };
            assert_eq!(loads(&mut rx).last(), Some(&(2, monet.clone())));
            // The Highlights arrive after the search started: too late.
            let late = deliver(&mut explorer, 1, Load::Highlights(vec![]), listing(1..3, 2));
            assert_eq!(late, None);
            assert_eq!(explorer.results.counts(), (0, 0));
            deliver(&mut explorer, 2, monet, listing(100..150, 120));
            assert_eq!(explorer.counts(), "50 / 120");
        }

        #[test]
        fn the_next_page_loads_near_the_end_of_the_list() {
            let (mut explorer, mut rx) = started();
            search(&mut explorer, "monet");
            loads(&mut rx);
            let first = Load::Search {
                query: Query::text("monet"),
                page: 1,
            };
            deliver(&mut explorer, 2, first, listing(0..PAGE_SIZE, 120));
            assert!(loads(&mut rx).is_empty(), "the cursor is at the top");
            let ahead = PAGE_SIZE as usize - LOAD_AHEAD - 1;
            for _ in 0..ahead - 1 {
                press(&mut explorer, char('j'));
            }
            assert!(loads(&mut rx).is_empty());
            press(&mut explorer, char('j'));
            let second = Load::Search {
                query: Query::text("monet"),
                page: 2,
            };
            assert_eq!(loads(&mut rx), [(2, second.clone())]);
            press(&mut explorer, char('j'));
            assert!(loads(&mut rx).is_empty(), "one page at a time");
            deliver(&mut explorer, 2, second, listing(50..100, 120));
            assert_eq!(explorer.counts(), "100 / 120");
            assert_eq!(selected_title(&explorer).as_deref(), Some("Artwork 40"));
        }

        #[test]
        fn a_failed_search_keeps_the_list_and_says_why() {
            let (mut explorer, _rx) = started();
            deliver(&mut explorer, 1, Load::Highlights(vec![]), listing(1..3, 2));
            search(&mut explorer, "monet");
            let failed = Load::Search {
                query: Query::text("monet"),
                page: 1,
            };
            deliver(&mut explorer, 2, failed, Err("the API is down".into()));
            assert_eq!(explorer.results.counts(), (2, 2));
            assert!(explorer.counts().ends_with("the API is down"));
        }

        #[test]
        fn an_empty_search_goes_back_to_the_highlights() {
            let (mut explorer, mut rx) = started();
            search(&mut explorer, "monet");
            search(&mut explorer, "");
            assert_eq!(
                loads(&mut rx).last(),
                Some(&(3, Load::Highlights(vec![24645, 16568])))
            );
        }

        #[test]
        fn the_status_bar_says_when_the_rate_limit_holds_things_up() {
            let (mut explorer, _rx) = started();
            explorer.update(Action::RateLimited(true)).unwrap();
            assert!(explorer.counts().ends_with("rate limited"));
        }

        #[test]
        fn a_search_past_the_record_cap_asks_for_a_narrower_one() {
            let (mut explorer, _rx) = started();
            search(&mut explorer, "a");
            let live = explorer.live.as_mut().unwrap();
            live.total = 52_000;
            let cap = MAX_RESULTS as u32;
            explorer.results.replace((0..cap).map(entry).collect());
            assert!(
                explorer
                    .counts()
                    .starts_with("1,000 / 52,000 · narrow the search")
            );
        }

        #[test]
        fn thousands_puts_commas_between_groups() {
            assert_eq!(thousands(0), "0");
            assert_eq!(thousands(999), "999");
            assert_eq!(thousands(1_000), "1,000");
            assert_eq!(thousands(1_234_567), "1,234,567");
        }

        /// A started explorer showing the Highlights `1..3`, with its requests drained.
        fn showing_highlights() -> (Explorer, UnboundedReceiver<Action>) {
            let (mut explorer, mut rx) = started();
            loads(&mut rx);
            deliver(&mut explorer, 1, Load::Highlights(vec![]), listing(1..3, 2));
            (explorer, rx)
        }

        fn ids(explorer: &Explorer) -> Vec<u32> {
            explorer.results.artworks().iter().map(Entry::id).collect()
        }

        #[test]
        fn a_and_d_search_by_the_artist_and_department_and_name_them() {
            let (mut explorer, mut rx) = showing_highlights();
            press(&mut explorer, char('A'));
            let artist = searched(&mut rx);
            assert_eq!(artist.filters.artist_id, Some(35));
            assert!(artist.filters.has_image, "the filters are kept");
            assert_eq!(artist.text, "");
            deliver(
                &mut explorer,
                2,
                Load::Search {
                    query: artist,
                    page: 1,
                },
                listing(10..12, 2),
            );
            assert_eq!(explorer.counts(), "2 / 2 · by Claude Monet");

            press(&mut explorer, char('D'));
            let department = searched(&mut rx);
            assert_eq!(department.filters.department_id.as_deref(), Some("PC-3"));
            assert_eq!(
                department.filters.artist_id,
                Some(35),
                "pivots narrow the list they start from"
            );
            assert_eq!(
                department.pivots(),
                ["by Claude Monet", "from Painting and Sculpture of Europe"]
            );
        }

        #[test]
        fn a_pivot_says_when_the_artwork_has_no_artist() {
            let (mut explorer, mut rx) = started();
            loads(&mut rx);
            let mut nobody = entry(1);
            nobody.summary.artist_id = None;
            let listed = Ok(Listing {
                entries: vec![nobody],
                total: 1,
                fallback: false,
            });
            deliver(&mut explorer, 1, Load::Highlights(vec![]), listed);
            press(&mut explorer, char('A'));
            assert!(loads(&mut rx).is_empty());
            assert_eq!(
                explorer.echo.as_deref(),
                Some("this artwork has no artist on record")
            );
        }

        #[test]
        fn backspace_goes_back_through_earlier_lists_with_their_cursors() {
            let (mut explorer, mut rx) = showing_highlights();
            search(&mut explorer, "monet");
            let monet = searched(&mut rx);
            let page = |page| Load::Search {
                query: monet.clone(),
                page,
            };
            deliver(&mut explorer, 2, page(1), listing(10..15, 5));
            press(&mut explorer, char('n'));
            press(&mut explorer, char('n'));
            press(&mut explorer, char('A'));
            let artist = searched(&mut rx);
            // A page still on its way for the pivot is dropped once we go back.
            press(&mut explorer, key(KeyCode::Backspace));
            assert_eq!(ids(&explorer), [10, 11, 12, 13, 14]);
            assert_eq!(explorer.selected().map(Entry::id), Some(12));
            assert_eq!(explorer.search.query(), "monet");
            assert_eq!(
                deliver(
                    &mut explorer,
                    3,
                    Load::Search {
                        query: artist,
                        page: 1
                    },
                    listing(20..22, 2)
                ),
                None
            );
            assert_eq!(ids(&explorer), [10, 11, 12, 13, 14]);

            press(&mut explorer, key(KeyCode::Backspace));
            assert_eq!(ids(&explorer), [1, 2]);
            assert_eq!(explorer.counts(), "Highlights · 2 artworks");
            press(&mut explorer, key(KeyCode::Backspace));
            assert_eq!(
                explorer.echo.as_deref(),
                Some("backspace · no earlier list")
            );
        }

        #[test]
        fn a_search_typed_while_one_loads_is_remembered_once() {
            let (mut explorer, mut rx) = showing_highlights();
            search(&mut explorer, "monet");
            search(&mut explorer, "seurat");
            let seurat = searched_last(&mut rx);
            deliver(
                &mut explorer,
                3,
                Load::Search {
                    query: seurat,
                    page: 1,
                },
                listing(30..31, 1),
            );
            assert_eq!(explorer.history.len(), 1, "only the Highlights");
        }

        /// The query of the last search asked for.
        fn searched_last(rx: &mut UnboundedReceiver<Action>) -> Query {
            match loads(rx).pop() {
                Some((_, Load::Search { query, page: 1 })) => query,
                other => panic!("expected a search, got {other:?}"),
            }
        }

        #[test]
        fn r_asks_for_a_surprise_and_another_replaces_it() {
            let (mut explorer, mut rx) = showing_highlights();
            explorer.seed = 42;
            press(&mut explorer, char('R'));
            let (generation, query, seed) = match loads(&mut rx).as_slice() {
                [(generation, Load::Surprise { query, seed })] => {
                    (*generation, query.clone(), *seed)
                }
                other => panic!("expected a surprise, got {other:?}"),
            };
            assert!(query.filters.has_image);
            let surprise = Load::Surprise {
                query: query.clone(),
                seed,
            };
            let shown = deliver(&mut explorer, generation, surprise, listing(77..78, 1));
            assert!(matches!(shown, Some(Action::ShowArtwork(e)) if e.id() == 77));
            assert_eq!(explorer.counts(), "Surprise");
            // No next page for a surprise.
            assert!(loads(&mut rx).is_empty());

            press(&mut explorer, char('R'));
            let again = loads(&mut rx);
            assert!(
                matches!(again.as_slice(), [(_, Load::Surprise { seed: s, .. })] if *s != seed),
                "a new seed: {again:?}"
            );
            assert_eq!(explorer.history.len(), 1, "surprises do not pile up");
            press(&mut explorer, key(KeyCode::Backspace));
            assert_eq!(ids(&explorer), [1, 2]);
        }

        #[test]
        fn the_saved_list_hides_the_results_while_they_load() {
            let (mut explorer, mut rx) = showing_highlights();
            press(&mut explorer, char('m'));
            search(&mut explorer, "monet");
            let monet = searched(&mut rx);
            press(&mut explorer, char('\''));
            assert_eq!(explorer.view(), View::Saved);
            deliver(
                &mut explorer,
                2,
                Load::Search {
                    query: monet,
                    page: 1,
                },
                listing(10..12, 2),
            );
            assert_eq!(ids(&explorer), [1], "the saved list still shows");
            assert_eq!(explorer.counts(), "Saved · 1 artwork");
            press(&mut explorer, char('\''));
            assert_eq!(ids(&explorer), [10, 11]);
        }
    }
}

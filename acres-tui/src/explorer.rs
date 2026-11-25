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
        catalog::{Listing, Load, Loaded},
        help::Help,
        home::Home,
        hud::Hud,
        pane_block,
        ramps::Ramps,
        record::Record,
        results::Results,
        search::Search,
        status::Status,
    },
    config::{Config, Context, KeyBindings, key_event_to_string, normalize},
    entry::Entry,
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

/// What the live list holds and what is on its way.
#[derive(Debug, Default)]
struct Live {
    highlights: Vec<u32>,
    /// The current search; increases with each new one.
    generation: u64,
    /// The query the list is for, or `None` for the Highlights.
    query: Option<String>,
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
    results: Results,
    art: Home,
    record: Record,
    hud: Hud,
    ramps: Ramps,
    help: Help,
    status: Status,
    areas: Areas,
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
            art,
            record,
            hud: Hud::new(),
            ramps: Ramps::new(),
            help: Help::default(),
            status: Status::default(),
            areas: Areas::default(),
            live,
            action_tx: None,
        }
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
            None => self.focus.context(),
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

    /// The keys the status bar suggests for where focus is.
    fn hints(&self) -> Vec<(String, &'static str)> {
        let context = self.context();
        let actions: &[Action] = match (self.overlay, self.focus) {
            (Some(Overlay::Help), _) => &[Action::MoveDown, Action::Cancel],
            (Some(Overlay::Ramps), _) => &[Action::Select, Action::Cancel],
            (None, Pane::Search) => &[Action::RunSearch, Action::EarlierQuery, Action::Cancel],
            (None, Pane::Results) => &[Action::MoveDown, Action::FocusArt, Action::FocusSearch],
            (None, Pane::Art) => &[Action::CycleArtMode, Action::ToggleZen, Action::NextArtwork],
            (None, Pane::Record) => &[Action::MoveDown, Action::NextArtwork, Action::FocusSearch],
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
        self.record.show(Some(artwork.clone()));
        Some(Action::ShowArtwork(artwork))
    }

    fn send(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(action);
        }
    }

    /// Starts a new live search for `query`, or the Highlights when it is empty.
    fn search_live(&mut self, query: String) {
        let Some(live) = &mut self.live else { return };
        live.generation += 1;
        live.loading = true;
        live.error = None;
        let load = if query.is_empty() {
            live.query = None;
            Load::Highlights(live.highlights.clone())
        } else {
            live.query = Some(query.clone());
            Load::Search { q: query, page: 1 }
        };
        let generation = live.generation;
        self.send(Action::Load { generation, load });
    }

    /// Asks for the next page when the cursor is near the end of a live search's list.
    fn load_more(&mut self) {
        let rows_below = self.results.rows_below();
        let (_, loaded) = self.results.counts();
        let Some(live) = &mut self.live else { return };
        let Some(q) = live.query.clone() else { return };
        let reachable = live.total.min(MAX_RESULTS);
        if live.loading || loaded as u64 >= reachable || rows_below > LOAD_AHEAD {
            return;
        }
        live.loading = true;
        let load = Load::Search {
            q,
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
            Load::Highlights(_) => true,
            Load::Search { page, .. } => page == 1,
        };
        live.total = total;
        live.fallback = fallback;
        if first {
            live.next_page = 2;
            let selected = self.results.replace(entries);
            self.record.show(selected.clone());
            let next = selected.map(Action::ShowArtwork);
            self.load_more();
            next
        } else {
            live.next_page += 1;
            self.results.append(entries);
            self.load_more();
            None
        }
    }

    /// What the status bar says about the list.
    fn counts(&self) -> String {
        let (shown, loaded) = self.results.counts();
        let Some(live) = &self.live else {
            return if shown == loaded {
                format!("{loaded} artworks")
            } else {
                format!("{shown} / {loaded} artworks")
            };
        };
        let mut parts = vec![match (&live.query, live.fallback) {
            (None, false) => format!("Highlights · {loaded} artworks"),
            (None, true) => format!("Recent public domain · {loaded}"),
            (Some(_), _) => format!("{} / {}", thousands(loaded as u64), thousands(live.total)),
        }];
        if live.query.is_some() && live.total > MAX_RESULTS && loaded as u64 >= MAX_RESULTS {
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
        let delta = match mouse.kind {
            MouseEventKind::ScrollDown => 1,
            MouseEventKind::ScrollUp => -1,
            MouseEventKind::Down(MouseButton::Left) => 0,
            _ => return Ok(None),
        };
        if let Some(overlay) = self.overlay {
            let area = match overlay {
                Overlay::Help => self.help.area,
                Overlay::Ramps => self.ramps.area,
            };
            if delta == 0 && !area.contains(at) {
                self.overlay = None;
            } else if delta != 0 {
                return Ok(self.scroll(delta));
            }
            return Ok(None);
        }
        if self.zen {
            return Ok(None);
        }
        let Some(pane) = self.areas.pane_at(at) else {
            return Ok(None);
        };
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
            (Pane::Record, 0) | (Pane::Search, 0) | (Pane::Art, 0) => {
                self.focus = pane;
                None
            }
            (Pane::Record, _) => {
                self.record.scroll(delta);
                None
            }
            // Scrolling over the art is kept for zooming (ADR 0013).
            (Pane::Search | Pane::Art, _) => None,
        })
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let next = match &action {
            Action::ToggleHelp => {
                if self.overlay == Some(Overlay::Help) {
                    self.overlay = None;
                } else {
                    self.help.open(&self.keybindings, self.focus.context());
                    self.overlay = Some(Overlay::Help);
                }
                None
            }
            Action::PickRamp => {
                self.overlay = Some(Overlay::Ramps);
                None
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
            Action::RunSearch => {
                let query = self.search.submit();
                self.focus = Pane::Results;
                if self.live.is_some() {
                    self.search_live(query);
                    None
                } else {
                    let moved = self.results.filter(&query);
                    if self.results.selected().is_none() {
                        self.record.show(None);
                    }
                    self.preview(moved)
                }
            }
            Action::Loaded(loaded) => self.loaded(loaded.clone()),
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
        // The art logs every action, and the HUD collects timings.
        self.art.update(action.clone())?;
        self.hud.update(action)?;
        Ok(next)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if self.zen {
            self.areas = Areas::default();
            self.art.draw(frame, area)?;
            self.hud.draw(frame, area)?;
        } else {
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

            self.status.counts = self.counts();
            self.status.hints = self.hints();
            self.status.echo = self.echo.clone();
            self.status.draw(frame, status)?;
        }
        match self.overlay {
            Some(Overlay::Help) => self.help.draw(frame, area)?,
            Some(Overlay::Ramps) => self.ramps.draw(frame, area)?,
            None => {}
        }
        Ok(())
    }
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
    use crate::components::test_support::{ARTWORKS_FIXTURE, render};

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
        assert_eq!(art, None, "scrolling over the art is kept for zoom");
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
                    ..Default::default()
                },
                iiif_url: "https://iiif.example/iiif/2".into(),
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
                q: "monet".into(),
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
                q: "monet".into(),
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
                q: "monet".into(),
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
                q: "monet".into(),
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
                    .starts_with("10,000 / 52,000 · narrow the search")
            );
        }

        #[test]
        fn thousands_puts_commas_between_groups() {
            assert_eq!(thousands(0), "0");
            assert_eq!(thousands(999), "999");
            assert_eq!(thousands(1_000), "1,000");
            assert_eq!(thousands(1_234_567), "1,234,567");
        }
    }
}

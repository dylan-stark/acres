//! The explorer screen: the search bar, results, art and record panes, the status bar, and the
//! overlays drawn over them (ADR 0013).
//!
//! The explorer owns focus. A key is looked up in the focused pane's context, then in `Global`,
//! and the action it maps to is sent like any other. Actions that change what is selected come
//! back as [`Action::IiifUpdateBaseUri`], which the image fetcher picks up.

use acres::artworks::ArtworkInfo;
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
        Component, help::Help, home::Home, hud::Hud, pane_block, ramps::Ramps, record::Record,
        results::Results, search::Search, status::Status,
    },
    config::{Config, Context, KeyBindings, key_event_to_string, normalize},
};

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
    action_tx: Option<UnboundedSender<Action>>,
}

impl Explorer {
    /// An explorer over `artworks`, with the first one selected and the results focused.
    pub fn new(artworks: acres::artworks::Artworks, config: &Config) -> Self {
        Self::with_art(artworks, config, Home::new())
    }

    /// The same, drawing the art with `art`, so tests and benches can fix the color depth.
    pub fn with_art(artworks: acres::artworks::Artworks, config: &Config, art: Home) -> Self {
        let results = Results::new(artworks);
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
            action_tx: None,
        }
    }

    /// The artwork selected now.
    pub fn selected(&self) -> Option<&ArtworkInfo> {
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

    /// Shows `artwork` if the selection moved to it, and asks for its image.
    fn preview(&mut self, artwork: Option<ArtworkInfo>) -> Option<Action> {
        let artwork = artwork?;
        self.record.show(Some(artwork.clone()));
        Some(Action::IiifUpdateBaseUri(artwork))
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
        if let (Some(tx), Some(artwork)) = (&self.action_tx, self.results.selected()) {
            tx.send(Action::IiifUpdateBaseUri(artwork.clone()))?;
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
                let moved = self.results.filter(&query);
                self.focus = Pane::Results;
                if self.results.selected().is_none() {
                    self.record.show(None);
                }
                self.preview(moved)
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

            let (shown, loaded) = self.results.counts();
            self.status.counts = if shown == loaded {
                format!("{loaded} artworks")
            } else {
                format!("{shown} / {loaded} artworks")
            };
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

#[cfg(test)]
mod tests {
    use charaoke::ColorDepth;

    use super::*;
    use crate::components::test_support::{ARTWORKS_FIXTURE, render};

    fn explorer() -> Explorer {
        let config = Config::new().expect("default config");
        let artworks = serde_json::from_str(ARTWORKS_FIXTURE).expect("fixture parses");
        Explorer::with_art(artworks, &config, Home::with_depth(ColorDepth::TrueColor))
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
        explorer.selected().map(|a| a.data.title.clone())
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
        assert!(
            matches!(next, Some(Action::IiifUpdateBaseUri(a)) if a.data.title == "The Bedroom")
        );
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
        assert!(matches!(next, Some(Action::IiifUpdateBaseUri(a)) if a.data.id == 24645));
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
        assert!(
            matches!(picked, Some(Action::IiifUpdateBaseUri(a)) if a.data.title == "Water Lilies")
        );
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
}

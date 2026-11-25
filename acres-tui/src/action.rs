use std::time::Duration;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumMessage};

use crate::{
    components::catalog::{Load, Loaded},
    entry::Entry,
};

/// Everything the TUI can be asked to do, from a key, the mouse, or another component.
///
/// The variants that keys can be bound to carry a `message`: the one-line description the help
/// overlay lists and the status bar echoes. Help lists them in declaration order, through
/// [`ActionDiscriminants`].
#[derive(
    Debug, Clone, PartialEq, Eq, Display, EnumMessage, EnumDiscriminants, Serialize, Deserialize,
)]
#[strum_discriminants(derive(PartialOrd, Ord, Hash))]
pub enum Action {
    // Keyed actions, in the order help lists them.
    #[strum(message = "toggle help")]
    ToggleHelp,
    #[strum(message = "search")]
    FocusSearch,
    #[strum(message = "focus the results")]
    FocusResults,
    #[strum(message = "focus the art")]
    FocusArt,
    #[strum(message = "focus the record")]
    FocusRecord,
    #[strum(message = "next pane")]
    FocusNext,
    #[strum(message = "previous pane")]
    FocusPrevious,
    #[strum(message = "next artwork")]
    NextArtwork,
    #[strum(message = "previous artwork")]
    PreviousArtwork,
    #[strum(message = "show the full image")]
    ShowFullImage,
    #[strum(message = "zen: art only")]
    ToggleZen,
    #[strum(message = "cycle art mode")]
    CycleArtMode,
    #[strum(message = "pick a ramp")]
    PickRamp,
    #[strum(message = "toggle timings")]
    ToggleHud,
    #[strum(message = "toggle logs")]
    ToggleLogs,
    #[strum(message = "quit, after asking")]
    AskQuit,
    #[strum(message = "quit without asking")]
    Quit,
    #[strum(message = "suspend")]
    Suspend,
    #[strum(message = "move down")]
    MoveDown,
    #[strum(message = "move up")]
    MoveUp,
    #[strum(message = "top")]
    Top,
    #[strum(message = "bottom")]
    Bottom,
    #[strum(message = "half a page down")]
    HalfPageDown,
    #[strum(message = "half a page up")]
    HalfPageUp,
    #[strum(message = "choose")]
    Select,
    #[strum(message = "run the search")]
    RunSearch,
    #[strum(message = "earlier query")]
    EarlierQuery,
    #[strum(message = "later query")]
    LaterQuery,
    #[strum(message = "clear")]
    ClearSearch,
    #[strum(message = "close")]
    Cancel,

    // Actions components send each other.
    Log(String),
    Tick,
    Render,
    Resize(u16, u16),
    Resume,
    ClearScreen,
    Error(String),
    /// Draws with the named ramp preset.
    SetRamp(String),
    /// A fetched image for artwork `id`, and how long the fetch took. `full` is false for the
    /// thumbnail fetched while browsing.
    #[strum(to_string = "UpdateImage(id: {id}, full: {full})")]
    UpdateImage {
        id: u32,
        image: Bytes,
        fetch: Duration,
        full: bool,
    },
    /// Stage durations for the timing HUD.
    #[strum(to_string = "{0}")]
    Timing(Timing),
    /// How long the last whole `terminal.draw` took.
    #[strum(to_string = "FrameTime([...])")]
    FrameTime(Duration),
    /// Shows this artwork: fetches its thumbnail after a short debounce.
    #[strum(to_string = "ShowArtwork")]
    #[serde(skip)]
    ShowArtwork(Entry),
    /// Asks for a list, as part of search `generation`.
    #[strum(to_string = "Load(generation: {generation})")]
    #[serde(skip)]
    Load {
        generation: u64,
        load: Load,
    },
    /// A list came back.
    #[strum(to_string = "Loaded")]
    #[serde(skip)]
    Loaded(Loaded),
    /// Whether a request is waiting on the rate limit now.
    RateLimited(bool),
}

impl Action {
    /// The one-line description shown in help and the status bar, for actions a key can do.
    pub fn description(&self) -> Option<&'static str> {
        self.get_message()
    }
}

/// One event in the timing HUD: a label, such as `resize 160x48`, and the stages it ran.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timing {
    pub label: String,
    pub stages: Vec<(String, Duration)>,
}

impl Timing {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            stages: Vec::new(),
        }
    }

    /// Adds `stage`, taking `duration`.
    pub fn stage(mut self, stage: impl Into<String>, duration: Duration) -> Self {
        self.stages.push((stage.into(), duration));
        self
    }
}

impl std::fmt::Display for Timing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)?;
        for (stage, duration) in &self.stages {
            write!(f, "  {stage} {:.2} ms", duration.as_secs_f64() * 1e3)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_actions_have_descriptions_and_internal_ones_do_not() {
        assert_eq!(Action::NextArtwork.description(), Some("next artwork"));
        assert_eq!(Action::Tick.description(), None);
    }
}

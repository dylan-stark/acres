use std::time::Duration;

use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants, EnumMessage};

use crate::{
    components::{
        art_view::FrameSize,
        catalog::{Load, Loaded},
        filters::Facets,
        home::Decoded,
    },
    entry::Entry,
    failure::Failure,
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
    #[strum(message = "expand to the full record")]
    ToggleFullRecord,
    #[strum(message = "open the full record")]
    OpenRecord,
    #[strum(message = "open on artic.edu")]
    OpenInBrowser,
    #[strum(message = "copy its link")]
    CopyLink,
    #[strum(message = "filters")]
    OpenFilters,
    #[strum(message = "cycle sort")]
    CycleSort,
    #[strum(message = "more by this artist")]
    MoreByArtist,
    #[strum(message = "more from this department")]
    MoreFromDepartment,
    #[strum(message = "surprise me")]
    Surprise,
    #[strum(message = "back to the previous list")]
    Back,
    #[strum(message = "save or unsave")]
    ToggleSaved,
    #[strum(message = "switch results and saved")]
    SwitchSaved,
    #[strum(message = "edit its note")]
    EditNote,
    #[strum(message = "zen: art only")]
    ToggleZen,
    #[strum(message = "cycle art mode")]
    CycleArtMode,
    #[strum(message = "zoom in")]
    ZoomIn,
    #[strum(message = "zoom out")]
    ZoomOut,
    #[strum(message = "pan left a cell")]
    PanLeft,
    #[strum(message = "pan down a cell")]
    PanDown,
    #[strum(message = "pan up a cell")]
    PanUp,
    #[strum(message = "pan right a cell")]
    PanRight,
    #[strum(message = "pan left a tenth")]
    PanLeftFar,
    #[strum(message = "pan down a tenth")]
    PanDownFar,
    #[strum(message = "pan up a tenth")]
    PanUpFar,
    #[strum(message = "pan right a tenth")]
    PanRightFar,
    #[strum(message = "fit the whole image")]
    FitArt,
    #[strum(message = "fill the frame")]
    FillArt,
    #[strum(message = "pick the frame size")]
    PickFrame,
    #[strum(message = "save this view")]
    SaveView,
    #[strum(message = "list the saved views")]
    ListViews,
    #[strum(message = "export the art to a file")]
    ExportArt,
    #[strum(message = "delete the saved view")]
    DeleteView,
    #[strum(message = "pick a ramp")]
    PickRamp,
    #[strum(message = "toggle timings")]
    ToggleHud,
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
    #[strum(message = "previous option")]
    PreviousOption,
    #[strum(message = "next option")]
    NextOption,
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
    Tick,
    Render,
    Resize(u16, u16),
    Resume,
    ClearScreen,
    /// Something the user asked for failed: shows it in the status bar.
    #[strum(to_string = "Failed")]
    #[serde(skip)]
    Failed(Failure),
    /// Draws with the named ramp preset.
    SetRamp(String),
    /// A decoded image for artwork `id`, and the HUD timing of its fetch and decode.
    #[strum(to_string = "UpdateImage(id: {id})")]
    #[serde(skip)]
    UpdateImage {
        id: u32,
        image: Decoded,
        timing: Timing,
    },
    /// Stage durations for the timing HUD.
    #[strum(to_string = "{0}")]
    Timing(Timing),
    /// How long the last whole `terminal.draw` took.
    #[strum(to_string = "FrameTime([...])")]
    FrameTime(Duration),
    /// Zooms by `steps`, in if positive, about the cell at `column`, `row` on the screen, as a
    /// mouse scroll over the art does.
    ZoomAt {
        column: u16,
        row: u16,
        steps: i8,
    },
    /// Pans the art by this many cells, as a mouse drag does.
    PanBy {
        cols: i32,
        rows: i32,
    },
    /// Draws the art on a frame of this size.
    SetFrame(FrameSize),
    /// Empties the art pane, for an artwork whose image is not at hand.
    ClearImage,
    /// Asks which of these artworks have their image in the disk cache (ADR 0018).
    #[strum(to_string = "CheckCached")]
    #[serde(skip)]
    CheckCached(Vec<Entry>),
    /// These artworks have their image in the disk cache, so their rows can say so.
    #[strum(to_string = "Cached")]
    #[serde(skip)]
    Cached(Vec<u32>),
    /// Shows this artwork: its image if it was seen recently or is cached, else nothing until
    /// the full image is asked for.
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
    /// Fetches the full record of artwork `id`.
    FetchRecord(u32),
    /// The full record of artwork `id` came back, or why it didn't.
    #[strum(to_string = "RecordLoaded(id: {id})")]
    #[serde(skip)]
    RecordLoaded {
        id: u32,
        result: Result<acres::artworks::Artwork, Failure>,
    },
    /// Lists the departments and artwork types for the filters.
    FetchFacets,
    /// The departments and artwork types came back, or why they didn't.
    #[strum(to_string = "FacetsLoaded")]
    #[serde(skip)]
    FacetsLoaded(Result<Facets, Failure>),
    /// Opens this link in the browser.
    OpenUrl(String),
    /// Puts this text on the clipboard.
    CopyText(String),
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

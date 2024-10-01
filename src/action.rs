use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    MoveDown,
    MoveUp,
    Select,
    ToggleArtworksList,
    Display(String),
    EnterImageDownload(String),
    ExitImageDownload(String),
    EnterSearch,
    ExitSearch,
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
}

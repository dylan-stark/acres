use bytes::Bytes;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    MoveDown,
    MoveUp,
    Select,
    EnterBrowseMode,
    EnterViewMode,
    LoadImage(String),
    //EnterImageDownload(String),
    ExitImageDownload(String),
    SetImage(Bytes),
    EnterSearch,
    ExitSearch,
    Tick,
    Render,
    Resize(u16, u16),
    Resume,
    Suspend,
    ToText(Bytes),
    Quit,
    ClearScreen,
    Error(String),
    Help,
}

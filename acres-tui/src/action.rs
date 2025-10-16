use acres::artworks::ArtworkInfo;
use bytes::Bytes;
use image_to_ascii_builder::Ascii;
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
    ToAscii(Bytes),
    ToText,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    #[strum(to_string = "View(id: {0})")]
    View(ArtworkInfo),
    #[strum(to_string = "RenderAscii([...])")]
    RenderAscii(Bytes),
    StartingRenderAscii,
    UpdateAscii(Ascii),
}

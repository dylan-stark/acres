use acres::artworks::ArtworkInfo;
use bytes::Bytes;
use image_to_ascii_builder::{Alphabet, Ascii, Font};
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Log(String),
    MoveDown,
    MoveUp,
    Select,
    EnterBrowseMode,
    EnterBrowseArtworksMode,
    EnterBrowseAlphabetsMode,
    EnterBrowseFontsMode,
    EnterViewMode,
    LoadImage(String),
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
    #[strum(to_string = "ViewAlphabet({0})")]
    ViewAlphabet(Alphabet),
    #[strum(to_string = "RenderAscii([...])")]
    StartingRenderAscii,
    UpdateAscii(Ascii),
    // Update base URI for IIIF tool
    #[strum(to_string = "IiifUpdateBaseUri(id: {0})")]
    IiifUpdateBaseUri(ArtworkInfo),
    // Request image using IIIF tool
    IiifRequestImage,
    // Update image for ImageToAsciiBuilder tool
    ImageToAsciiBuilderUpdateAlphabet(Alphabet),
    ImageToAsciiBuilderUpdateFont(Font),
    ImageToAsciiBuilderUpdateImage(Bytes),
    ImageToAsciiBuilderBuildAscii,
}

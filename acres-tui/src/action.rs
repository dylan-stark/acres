use acres::artworks::ArtworkInfo;
use std::time::Duration;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    ToggleLogs,
    ToggleHud,
    Log(String),
    MoveDown,
    MoveUp,
    Select,
    EnterBrowseMode,
    EnterBrowseArtworksMode,
    EnterBrowseRampsMode,
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
    /// Draws with the named ramp preset.
    SetRamp(String),
    /// A fetched image for artwork `id`, and how long the fetch took.
    #[strum(to_string = "UpdateImage(id: {id})")]
    UpdateImage {
        id: u32,
        image: Bytes,
        fetch: Duration,
    },
    /// Stage durations for the timing HUD.
    #[strum(to_string = "{0}")]
    Timing(Timing),
    /// How long the last whole `terminal.draw` took.
    #[strum(to_string = "FrameTime([...])")]
    FrameTime(Duration),
    // Update base URI for IIIF tool
    #[strum(to_string = "IiifUpdateBaseUri(id: {0})")]
    IiifUpdateBaseUri(ArtworkInfo),
    // Request image using IIIF tool
    IiifRequestImage,
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

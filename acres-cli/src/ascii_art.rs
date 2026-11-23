//! The `ascii-art` subcommand: converts an image to ASCII art with `charaoke`.

use std::{
    fmt,
    io::{self, IsTerminal, Read, Write},
    num::NonZeroU32,
    time::{Duration, Instant},
};

use charaoke::{Cells, ColorDepth, Converter, Grid, PixelFormat, Ramp};
use color_eyre::{Result, eyre::WrapErr};

/// Width used when stdout is not a terminal, in columns.
pub const DEFAULT_WIDTH: u32 = 80;

/// The `--color` choice.
#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorChoice {
    /// Color when stdout is a terminal and `NO_COLOR` is unset, at the depth the terminal reports.
    #[default]
    Auto,
    /// Always 24-bit color.
    #[value(name = "truecolor")]
    TrueColor,
    /// Always 256-color.
    #[value(name = "256")]
    Ansi256,
    /// No color.
    Never,
}

impl ColorChoice {
    /// The color depth to write, or `None` for plain text.
    pub fn depth(self, is_tty: bool, env: impl Fn(&str) -> Option<String>) -> Option<ColorDepth> {
        match self {
            ColorChoice::TrueColor => Some(ColorDepth::TrueColor),
            ColorChoice::Ansi256 => Some(ColorDepth::Ansi256),
            ColorChoice::Never => None,
            ColorChoice::Auto => {
                if !is_tty || env("NO_COLOR").is_some_and(|v| !v.is_empty()) {
                    return None;
                }
                if env("TERM").as_deref() == Some("dumb") {
                    return None;
                }
                match env("COLORTERM").as_deref() {
                    Some("truecolor" | "24bit") => Some(ColorDepth::TrueColor),
                    _ => Some(ColorDepth::Ansi256),
                }
            }
        }
    }
}

/// Parses `--ramp`: a preset name, or else the ramp's own characters from darkest to brightest.
pub fn parse_ramp(value: &str) -> Result<Ramp, String> {
    Ramp::preset(value)
        .map(Ok)
        .unwrap_or_else(|| Ramp::new(value).map_err(|e| e.to_string()))
}

/// The default `--width`: the terminal's width, or [`DEFAULT_WIDTH`] when stdout is not one.
pub fn default_width() -> u32 {
    if io::stdout().is_terminal()
        && let Ok((cols, _)) = crossterm::terminal::size()
        && cols > 0
    {
        u32::from(cols)
    } else {
        DEFAULT_WIDTH
    }
}

/// Stage durations, in the order they ran.
#[derive(Debug, Default)]
pub struct Timings(Vec<(&'static str, Duration)>);

impl Timings {
    /// Runs `f`, recording how long it took as `stage`.
    pub fn time<T>(&mut self, stage: &'static str, f: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let value = f();
        self.0.push((stage, start.elapsed()));
        value
    }

    /// Records `stage` as taking `duration`.
    pub fn push(&mut self, stage: &'static str, duration: Duration) {
        self.0.push((stage, duration));
    }
}

impl fmt::Display for Timings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (stage, duration) in &self.0 {
            writeln!(f, "{stage:<7} {:>9.3} ms", duration.as_secs_f64() * 1e3)?;
        }
        Ok(())
    }
}

/// What to draw, from the command line.
pub struct Options {
    /// Characters from darkest to brightest, with the brightness offset applied.
    pub ramp: Ramp,
    /// Width of the art in columns.
    pub width: NonZeroU32,
    /// Color depth, or `None` for plain text.
    pub depth: Option<ColorDepth>,
    /// Whether to print stage timings to stderr.
    pub timings: bool,
}

/// Reads an image from `input`, writes it as ASCII art to `out`, then the timings to stderr.
pub fn run(mut input: impl Read, out: &mut impl Write, options: Options) -> Result<()> {
    let start = Instant::now();
    let mut timings = Timings::default();

    let bytes = timings.time("read", || {
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes).map(|_| bytes)
    });
    let bytes = bytes.context("failed to read input image")?;

    let image = timings
        .time("decode", || {
            image::load_from_memory(&bytes).map(image::DynamicImage::into_rgb8)
        })
        .context("failed to decode input image")?;
    let (Some(w), Some(h)) = (
        NonZeroU32::new(image.width()),
        NonZeroU32::new(image.height()),
    ) else {
        color_eyre::eyre::bail!("input image is empty");
    };

    let mut converter = Converter::new(options.ramp);
    timings
        .time("load", || {
            converter.set_image(image.as_raw(), w, h, PixelFormat::Rgb8)
        })
        .context("failed to load image")?;

    let mut cells = Cells::default();
    timings.time("render", || {
        converter.render(Grid::fit(w, h, options.width), &mut cells)
    });

    timings
        .time("write", || {
            let mut text = String::new();
            match options.depth {
                Some(depth) => cells.write_ansi(depth, &mut text),
                None => cells.write_plain(&mut text),
            }
            text.push('\n');
            out.write_all(text.as_bytes())?;
            out.flush()
        })
        .context("failed to write art")?;
    timings.push("total", start.elapsed());

    if options.timings {
        eprint!("{timings}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    #[test]
    fn auto_reads_the_terminal() {
        let auto = ColorChoice::Auto;
        assert_eq!(
            auto.depth(true, env(&[("COLORTERM", "truecolor")])),
            Some(ColorDepth::TrueColor)
        );
        assert_eq!(
            auto.depth(true, env(&[("TERM", "xterm-256color")])),
            Some(ColorDepth::Ansi256)
        );
        assert_eq!(auto.depth(true, env(&[])), Some(ColorDepth::Ansi256));
        assert_eq!(auto.depth(true, env(&[("TERM", "dumb")])), None);
    }

    #[test]
    fn auto_turns_off_for_no_color_or_pipes() {
        let auto = ColorChoice::Auto;
        let truecolor = [("COLORTERM", "truecolor")];
        assert_eq!(auto.depth(false, env(&truecolor)), None);
        assert_eq!(
            auto.depth(true, env(&[("COLORTERM", "truecolor"), ("NO_COLOR", "1")])),
            None
        );
        assert_eq!(
            auto.depth(true, env(&[("COLORTERM", "truecolor"), ("NO_COLOR", "")])),
            Some(ColorDepth::TrueColor)
        );
    }

    #[test]
    fn forced_depths_ignore_the_terminal() {
        let none = env(&[("NO_COLOR", "1")]);
        assert_eq!(
            ColorChoice::TrueColor.depth(false, &none),
            Some(ColorDepth::TrueColor)
        );
        assert_eq!(
            ColorChoice::Ansi256.depth(false, &none),
            Some(ColorDepth::Ansi256)
        );
        assert_eq!(ColorChoice::Never.depth(true, env(&[])), None);
    }

    #[test]
    fn ramp_takes_a_preset_or_characters() {
        assert_eq!(parse_ramp("fast"), Ok(Ramp::preset("fast").unwrap()));
        assert_eq!(parse_ramp(" .#"), Ok(Ramp::new(" .#").unwrap()));
        assert!(parse_ramp("").is_err());
    }
}

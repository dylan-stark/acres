use std::fmt;

use crate::Error;

/// An ordered run of ASCII characters, from darkest to brightest.
///
/// A cell's average brightness picks one character, so a brighter cell gets a denser glyph (the
/// usual look on a dark terminal background). Lookup goes through a 256-entry table built once,
/// so choosing a character costs one indexed load per cell.
///
/// ```rust
/// use charaoke::Ramp;
///
/// let ramp = Ramp::new(" .:#").unwrap();
/// assert_eq!(ramp.lookup(0), b' ');
/// assert_eq!(ramp.lookup(255), b'#');
/// ```
#[derive(Clone)]
pub struct Ramp {
    chars: Box<str>,
    brightness_offset: u8,
    table: [u8; 256],
}

impl Ramp {
    /// The default ramp, ten characters from space to `@`.
    pub const DEFAULT: &'static str = " .:-=+*#%@";

    /// Named ramps, each ordered darkest first.
    ///
    /// These are the character sets `image-to-ascii` 0.7 ships as alphabets, sorted by how many
    /// pixels each glyph lights in its default font (bitocra-13), so they shade the same way that
    /// font draws them. `alphabet` is the fullest and shades the most finely.
    pub const PRESETS: &'static [(&'static str, &'static str)] = &[
        (
            "alphabet",
            r#" '.`-",_!:=+\()/;^i|lr~Lc{}*1Tfjvx47CFJnu#IYstz?%&KVXhkmow3EGHPSUapq2569Zey$AbdDMNORWg08BQ[]@"#,
        ),
        ("fast", "#"),
        (
            "letters",
            " ilrLcTfjvxCFJnuIYstzKVXhkmowEGHPSUapqZeyAbdDMNORWgBQ",
        ),
        ("lowercase", " ilrcfjvxnustzhkmowapqeybdg"),
        ("minimal", r" ._!/\^*"),
        ("symbols", r#" '.`-",_!:=+\();^|~{}*#?%&$[]@"#),
        ("uppercase", " LTCFJIYKVXEGHPSUZADMNORWBQ"),
    ];

    /// The preset named `name` (see [`PRESETS`](Self::PRESETS)), if there is one.
    ///
    /// ```rust
    /// use charaoke::Ramp;
    ///
    /// assert_eq!(Ramp::preset("minimal").unwrap().chars(), r" ._!/\^*");
    /// assert!(Ramp::preset("nope").is_none());
    /// ```
    pub fn preset(name: &str) -> Option<Self> {
        Self::PRESETS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, chars)| Self::new(chars).expect("presets are valid"))
    }

    /// Builds a ramp from `chars`, darkest first.
    ///
    /// Fails if `chars` is empty or holds anything but printable ASCII (space to `~`), since
    /// every cell must be exactly one byte wide.
    pub fn new(chars: &str) -> Result<Self, Error> {
        if chars.is_empty() {
            return Err(Error::EmptyRamp);
        }
        if let Some(c) = chars.chars().find(|c| !(' '..='~').contains(c)) {
            return Err(Error::InvalidRampChar(c));
        }
        Ok(Self {
            chars: chars.into(),
            brightness_offset: 0,
            table: table(chars.as_bytes(), 0),
        })
    }

    /// The same ramp, darkened by subtracting `offset` from every cell's brightness before lookup.
    ///
    /// The offset is built into the lookup table, so it costs nothing per cell.
    ///
    /// ```rust
    /// use charaoke::Ramp;
    ///
    /// let ramp = Ramp::new(" #").unwrap().with_brightness_offset(100);
    /// assert_eq!(ramp.lookup(200), b' ');
    /// assert_eq!(ramp.lookup(255), b'#');
    /// ```
    pub fn with_brightness_offset(self, offset: u8) -> Self {
        Self {
            table: table(self.chars.as_bytes(), offset),
            brightness_offset: offset,
            ..self
        }
    }

    /// The offset set by [`with_brightness_offset`](Self::with_brightness_offset), 0 by default.
    pub fn brightness_offset(&self) -> u8 {
        self.brightness_offset
    }

    /// The character for a cell of average brightness `luma` (0 is black, 255 is white).
    #[inline]
    pub fn lookup(&self, luma: u8) -> u8 {
        self.table[usize::from(luma)]
    }

    /// The ramp's characters, darkest first.
    pub fn chars(&self) -> &str {
        &self.chars
    }
}

/// Maps each brightness to a character, rounding to the nearest step so both ends of the ramp get
/// a fair share of values.
fn table(bytes: &[u8], offset: u8) -> [u8; 256] {
    let last = bytes.len() - 1;
    let mut table = [0; 256];
    for (luma, entry) in table.iter_mut().enumerate() {
        let luma = luma.saturating_sub(usize::from(offset));
        *entry = bytes[(luma * last + 127) / 255];
    }
    table
}

impl PartialEq for Ramp {
    fn eq(&self, other: &Self) -> bool {
        self.chars == other.chars && self.brightness_offset == other.brightness_offset
    }
}

impl Eq for Ramp {}

impl Default for Ramp {
    fn default() -> Self {
        Self::new(Self::DEFAULT).expect("default ramp is valid")
    }
}

impl fmt::Debug for Ramp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ramp")
            .field("chars", &self.chars)
            .field("brightness_offset", &self.brightness_offset)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ends_map_to_first_and_last_chars() {
        let ramp = Ramp::default();
        assert_eq!(ramp.lookup(0), b' ');
        assert_eq!(ramp.lookup(255), b'@');
    }

    #[test]
    fn is_monotonic() {
        let ramp = Ramp::default();
        let position = |c: u8| ramp.chars().bytes().position(|r| r == c).unwrap();
        for luma in 1..=255u8 {
            assert!(position(ramp.lookup(luma - 1)) <= position(ramp.lookup(luma)));
        }
    }

    #[test]
    fn uses_every_char() {
        let ramp = Ramp::default();
        for c in ramp.chars().bytes() {
            assert!(
                (0..=255u8).any(|luma| ramp.lookup(luma) == c),
                "{}",
                c as char
            );
        }
    }

    #[test]
    fn single_char_ramp_is_constant() {
        let ramp = Ramp::new("#").unwrap();
        assert!((0..=255u8).all(|luma| ramp.lookup(luma) == b'#'));
    }

    #[test]
    fn presets_are_valid_and_distinct() {
        for (i, (name, chars)) in Ramp::PRESETS.iter().enumerate() {
            let ramp = Ramp::preset(name).unwrap();
            assert_eq!(ramp.chars(), *chars);
            let mut seen: Vec<u8> = chars.bytes().collect();
            seen.sort_unstable();
            seen.dedup();
            assert_eq!(seen.len(), chars.len(), "{name} repeats a character");
            assert!(
                Ramp::PRESETS[..i].iter().all(|(n, _)| n != name),
                "{name} is listed twice"
            );
        }
    }

    #[test]
    fn brightness_offset_darkens() {
        let plain = Ramp::default();
        let dark = Ramp::default().with_brightness_offset(64);
        assert_eq!(dark.brightness_offset(), 64);
        for luma in 0..=255u8 {
            assert_eq!(dark.lookup(luma), plain.lookup(luma.saturating_sub(64)));
        }
        assert_ne!(dark, plain);
        assert_eq!(dark.with_brightness_offset(0), plain);
    }

    #[test]
    fn rejects_empty_and_non_ascii() {
        assert!(matches!(Ramp::new(""), Err(Error::EmptyRamp)));
        assert!(matches!(Ramp::new(" .█"), Err(Error::InvalidRampChar('█'))));
        assert!(matches!(
            Ramp::new(" \t"),
            Err(Error::InvalidRampChar('\t'))
        ));
    }
}

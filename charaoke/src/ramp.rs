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
    table: [u8; 256],
}

impl Ramp {
    /// The default ramp, ten characters from space to `@`.
    pub const DEFAULT: &'static str = " .:-=+*#%@";

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
        let bytes = chars.as_bytes();
        let last = bytes.len() - 1;
        let mut table = [0; 256];
        for (luma, entry) in table.iter_mut().enumerate() {
            // Rounds to the nearest step, so both ends of the ramp get a fair share of values.
            *entry = bytes[(luma * last + 127) / 255];
        }
        Ok(Self {
            chars: chars.into(),
            table,
        })
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

impl Default for Ramp {
    fn default() -> Self {
        Self::new(Self::DEFAULT).expect("default ramp is valid")
    }
}

impl fmt::Debug for Ramp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Ramp").field(&self.chars).finish()
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
    fn rejects_empty_and_non_ascii() {
        assert!(matches!(Ramp::new(""), Err(Error::EmptyRamp)));
        assert!(matches!(Ramp::new(" .█"), Err(Error::InvalidRampChar('█'))));
        assert!(matches!(
            Ramp::new(" \t"),
            Err(Error::InvalidRampChar('\t'))
        ));
    }
}

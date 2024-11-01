#[derive(Copy, Clone, Default)]
pub struct ArtBuilder {
    width: u16,
    height: u16,
}

impl ArtBuilder {
    pub fn of_size(mut self, width: u16, height: u16) -> ArtBuilder {
        self.width = width;
        self.height = height;
        self
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_default() {
        ArtBuilder::default();
    }

    #[test]
    fn test_size_sets_width_and_height() {
        let (some_width, some_height) = (10, 10);

        let builder = ArtBuilder::default().of_size(some_width, some_height);

        assert_eq!(builder.width, some_width);
        assert_eq!(builder.height, some_height);
    }

    #[test]
    fn test_size_updates_width_and_height() {
        let (some_width, some_height) = (10, 10);
        let builder = ArtBuilder::default().of_size(42, 42);

        let builder = builder.of_size(some_width, some_height);

        assert_eq!(builder.width, some_width);
        assert_eq!(builder.height, some_height);
    }
}

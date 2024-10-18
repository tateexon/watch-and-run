#[derive(Default)]
pub struct RecentStrings {
    data: [String; 2],
    index: usize,
}

impl RecentStrings {
    /// Adds a new string to the struct, replacing the older string.
    pub fn add(&mut self, s: String) {
        // Update the index to point to the older string
        self.index = (self.index + 1) % 2;
        self.data[self.index] = s;
    }

    /// Checks if the given string matches either of the stored strings.
    pub fn contains(&self, s: &str) -> bool {
        self.data[0] == s || self.data[1] == s
    }
}

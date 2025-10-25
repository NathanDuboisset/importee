/// Utilities for representing and manipulating dotted Python-like module paths.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModulePath {
    /// Ordered path segments, e.g., ["foo", "bar"] for "foo.bar".
    segments: Vec<String>,
}

impl ModulePath {
    /// Create a new ModulePath from concrete segments.
    pub fn new(segments: Vec<String>) -> Self {
        ModulePath { segments }
    }

    /// Borrow the inner segments.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Whether this ModulePath has no segments.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Format as dotted identifier string (e.g., "foo.bar").
    pub fn to_dotted(&self) -> String {
        self.segments.join(".")
    }

    /// Build a PathBuf corresponding to this module as a directory path.
    pub fn to_dir_pathbuf(&self) -> std::path::PathBuf {
        let mut buf = std::path::PathBuf::new();
        for seg in &self.segments {
            buf.push(seg);
        }
        buf
    }

    /// Return a new ModulePath with an extra trailing segment appended.
    pub fn append(&self, segment: String) -> ModulePath {
        let mut segs = self.segments.clone();
        segs.push(segment);
        ModulePath::new(segs)
    }

    /// Split off the last segment, returning (last, parent).
    pub fn split_last(&self) -> Option<(String, ModulePath)> {
        if self.segments.is_empty() {
            return None;
        }
        let mut parent = self.segments.clone();
        let leaf = parent.pop().unwrap();
        Some((leaf, ModulePath::new(parent)))
    }

    /// Interpret this ModulePath as a file module and return its .py file path.
    /// If empty, returns an empty PathBuf.
    pub fn file_path(&self) -> std::path::PathBuf {
        if let Some((leaf, parent)) = self.split_last() {
            let mut buf = parent.to_dir_pathbuf();
            buf.push(format!("{}.py", leaf));
            buf
        } else {
            std::path::PathBuf::new()
        }
    }

    /// Build a ModulePath from a dotted string (e.g., "foo.bar").
    /// Empty or all-dot strings produce an empty ModulePath.
    pub fn from_dotted(dotted: &str) -> ModulePath {
        let segments: Vec<String> = dotted
            .split('.')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        ModulePath::new(segments)
    }

    /// Resolve an import string against a current module path.
    /// - Absolute imports (no leading '.') return the absolute path (e.g., "foo.nothing").
    /// - Relative imports (leading dots) climb up by dot count and then append the remainder.
    ///   Example: current="foo.bar", import=".other" => "foo.other".
    pub fn from_import(current: &ModulePath, import: &str) -> ModulePath {
        let dot_prefix = import.chars().take_while(|&c| c == '.').count();
        let remainder = &import[dot_prefix..];

        if dot_prefix == 0 {
            // Absolute import
            return ModulePath::from_dotted(remainder);
        }

        // Relative import: climb up `dot_prefix` levels (best-effort if overflows)
        let base_len = current.segments.len().saturating_sub(dot_prefix);
        let mut segments = current.segments[..base_len].to_vec();

        if !remainder.is_empty() {
            segments.extend(
                remainder
                    .split('.')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
            );
        }
        ModulePath::new(segments)
    }
}

#[cfg(test)]
mod tests {
    use super::ModulePath;

    #[test]
    fn dotted_roundtrip_and_path() {
        let mp = ModulePath::from_dotted("foo.bar");
        assert_eq!(mp.segments(), &["foo", "bar"]);
        assert_eq!(mp.to_dotted(), "foo.bar");
    }

    #[test]
    fn from_import_absolute() {
        let cur = ModulePath::from_dotted("foo.bar");
        let out = ModulePath::from_import(&cur, "foo.nothing");
        assert_eq!(out.to_dotted(), "foo.nothing");
    }

    #[test]
    fn from_import_relative_single_dot() {
        let cur = ModulePath::from_dotted("foo.bar");
        let out = ModulePath::from_import(&cur, ".other");
        assert_eq!(out.to_dotted(), "foo.other");
    }

    #[test]
    fn from_import_relative_multi_dot() {
        let cur = ModulePath::from_dotted("a.b.c");
        let out = ModulePath::from_import(&cur, "..d");
        assert_eq!(out.to_dotted(), "a.d");
    }
}

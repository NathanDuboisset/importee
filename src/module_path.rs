use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

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

    /// Whether this ModulePath starts with the given base path.
    pub fn starts_with(&self, base: &ModulePath) -> bool {
        if base.segments.len() > self.segments.len() {
            return false;
        }
        self.segments
            .iter()
            .zip(base.segments.iter())
            .all(|(a, b)| a == b)
    }

    /// Return the relative ModulePath by stripping the given base prefix.
    /// Example: self="importee.path.api", base="importee" => Some("path.api").
    /// If `self` doesn't start with `base`, returns None.
    pub fn relative_from(&self, base: &ModulePath) -> Option<ModulePath> {
        if !self.starts_with(base) {
            return None;
        }
        let rest = self.segments[base.segments.len()..].to_vec();
        Some(ModulePath::new(rest))
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

impl Serialize for ModulePath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_dotted())
    }
}

impl<'de> Deserialize<'de> for ModulePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ModulePathVisitor;
        impl<'de> Visitor<'de> for ModulePathVisitor {
            type Value = ModulePath;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter
                    .write_str("a dotted string, array of segments, or an object with 'segments'")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ModulePath::from_dotted(value))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut segments: Vec<String> = Vec::new();
                while let Some(elem) = seq.next_element::<String>()? {
                    if !elem.is_empty() {
                        segments.push(elem);
                    }
                }
                Ok(ModulePath::new(segments))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut segments: Option<Vec<String>> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "segments" => {
                            segments = Some(map.next_value::<Vec<String>>()?);
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                Ok(ModulePath::new(segments.unwrap_or_default()))
            }
        }

        deserializer.deserialize_any(ModulePathVisitor)
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

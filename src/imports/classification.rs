use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::module_path::ModulePath;
use dashmap::DashMap;

#[derive(Clone)]
pub struct ImportResolver {
    cache: Arc<DashMap<String, bool>>,
    root_dir: PathBuf,
    root_module: Option<String>,
    /// Cached prefix string for performance (root_module + ".")
    root_module_prefix: Option<String>,
}

impl Default for ImportResolver {
    fn default() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            root_dir: PathBuf::new(),
            root_module: None,
            root_module_prefix: None,
        }
    }
}

impl ImportResolver {
    pub fn new(root_dir: impl Into<PathBuf>, root_module: Option<String>, _verbose: bool) -> Self {
        let root_module_prefix = root_module.as_ref().map(|m| format!("{}.", m));
        Self {
            cache: Arc::new(DashMap::new()),
            root_dir: root_dir.into(),
            root_module,
            root_module_prefix,
        }
    }

    /// Project root directory for resolution (used for caching paths and lookups)
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Returns true if the dotted module path exists under the configured root directory,
    /// without requiring it to be prefixed by the root module name.
    pub fn module_exists_under_root(&self, dotted: &str) -> bool {
        if dotted.is_empty() {
            return false;
        }
        // Accept both root-prefixed and project-relative dotted names
        // Use cached prefix to avoid string allocation
        let dotted_rel = if let Some(root_mod) = &self.root_module {
            if dotted == root_mod {
                ""
            } else if let Some(prefix) = &self.root_module_prefix {
                if let Some(stripped) = dotted.strip_prefix(prefix.as_str()) {
                    stripped
                } else {
                    dotted
                }
            } else {
                dotted
            }
        } else {
            dotted
        };
        if dotted_rel.is_empty() {
            return self.root_dir.join("__init__.py").exists();
        }
        let rel = dotted_rel.replace('.', "/");
        let file = self.root_dir.join(format!("{}.py", rel));
        if file.exists() {
            return true;
        }
        self.root_dir.join(&rel).join("__init__.py").exists()
    }

    /// Resolve an import string potentially missing the project root module prefix by
    /// trying progressively longer prefixes from the current module's parent.
    /// - Relative imports (starting with '.') are handled like Python's semantics.
    /// - Absolute-like imports are first tried as-is, then prefixed with the beginning
    ///   of the current module path (e.g., root, then root.sub, ...).
    pub fn resolve_import(&self, current_module: &ModulePath, import: &str) -> ModulePath {
        if import.starts_with('.') {
            return ModulePath::from_import(current_module, import);
        }

        // If the import already starts with the root module, do not prefix further
        // Use cached prefix to avoid string allocation
        if let Some(root_mod) = &self.root_module {
            if import == root_mod {
                return ModulePath::from_dotted(import);
            }
            if let Some(prefix) = &self.root_module_prefix {
                if import.starts_with(prefix.as_str()) {
                    return ModulePath::from_dotted(import);
                }
            }
        }

        // Try as-is first (project-relative)
        if self.module_exists_under_root(import) {
            return ModulePath::from_dotted(import);
        }

        // Walk up from the parent module, progressively prepending its prefixes
        let parent = current_module
            .split_last()
            .map(|(_, p)| p)
            .unwrap_or_else(|| ModulePath::new(vec![]));
        let parent_segments = parent.segments().to_vec();
        for i in 1..=parent_segments.len() {
            let mut combined: Vec<String> = parent_segments[0..i].to_vec();
            combined.extend(ModulePath::from_dotted(import).segments().iter().cloned());
            let candidate = combined.join(".");
            let exists = if self.root_module.is_some() {
                self.is_local_dotted(&candidate)
            } else {
                self.module_exists_under_root(&candidate)
            };
            if exists {
                return ModulePath::from_dotted(&candidate);
            }
        }

        // Fallback to the original absolute form
        ModulePath::from_dotted(import)
    }
    /// Returns true if the dotted module path points inside the project root.
    pub fn is_local_dotted(&self, dotted: &str) -> bool {
        if dotted.is_empty() {
            return false;
        }
        // Fast path: check cache (lock-free with DashMap)
        if let Some(found) = self.cache.get(dotted) {
            return *found;
        }
        // Resolve and cache
        let mut is_local = self.exists_in_root(dotted);
        if !is_local {
            // Also consider modules that exist under root without explicit root prefix
            is_local = self.module_exists_under_root(dotted);
        }
        // Insert into cache (lock-free)
        self.cache.insert(dotted.to_string(), is_local);
        is_local
    }

    /// Returns true if the module exists under root.
    fn exists_in_root(&self, dotted: &str) -> bool {
        if let Some(root_mod) = &self.root_module {
            if dotted == root_mod {
                return self.root_dir.join("__init__.py").exists();
            }
            // Use cached prefix to avoid string allocation
            if let Some(prefix) = &self.root_module_prefix {
                if let Some(stripped) = dotted.strip_prefix(prefix.as_str()) {
                    let rel = stripped.replace('.', "/");
                    let file = self.root_dir.join(format!("{}.py", rel));
                    if file.exists() {
                        return true;
                    }
                    return self.root_dir.join(&rel).join("__init__.py").exists();
                }
            }
            // Not under root module => external
            return false;
        }
        // Fallback: treat dotted path as project-relative
        let rel = dotted.replace('.', "/");
        let file = self.root_dir.join(format!("{}.py", rel));
        if file.exists() {
            return true;
        }
        self.root_dir.join(&rel).join("__init__.py").exists()
    }

    /// Helper for ModulePath input.
    pub fn is_local_module(&self, module: &ModulePath) -> bool {
        self.is_local_dotted(&module.to_dotted())
    }

    /// Classify a module as local or external, with a human-readable reason for external.
    pub fn classify_module(&self, module: &ModulePath) -> (bool, String) {
        let dotted = module.to_dotted();
        if self.is_local_dotted(&dotted) {
            return (true, String::new());
        }

        // Compute why it's considered external
        if let Some(root_mod) = &self.root_module {
            // Use cached prefix to avoid string allocation
            let has_prefix = if let Some(prefix) = &self.root_module_prefix {
                dotted == *root_mod || dotted.starts_with(prefix.as_str())
            } else {
                dotted == *root_mod
            };
            if !has_prefix {
                return (false, format!("not in root module '{}'", root_mod));
            }
            // Has correct prefix but path missing
            let rel = if dotted == *root_mod {
                String::from("__init__.py")
            } else {
                format!(
                    "{}/__init__.py",
                    dotted[root_mod.len() + 1..].replace('.', "/")
                )
            };
            let file = if dotted == *root_mod {
                self.root_dir.join("__init__.py")
            } else {
                self.root_dir
                    .join(&dotted[root_mod.len() + 1..].replace('.', "/"))
            };
            return (
                false,
                format!(
                    "path not found under root: {} (or {})",
                    file.with_extension("py").to_string_lossy(),
                    self.root_dir.join(rel).to_string_lossy()
                ),
            );
        }

        // No root module configured; fallback to cwd-based path check
        let rel = dotted.replace('.', "/");
        let file = self.root_dir.join(format!("{}.py", rel));
        (
            false,
            format!("path not found under cwd: {}", file.to_string_lossy()),
        )
    }
}

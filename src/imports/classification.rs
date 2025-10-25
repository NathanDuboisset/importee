use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::module_path::ModulePath;

#[derive(Clone, Default)]
pub struct ImportResolver {
    cache: Arc<RwLock<HashMap<String, bool>>>,
    root_dir: PathBuf,
    root_module: Option<String>,
}

impl ImportResolver {
    pub fn new(root_dir: impl Into<PathBuf>, root_module: Option<String>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            root_dir: root_dir.into(),
            root_module,
        }
    }

    /// Returns true if the dotted module path points inside the project root.
    pub fn is_local_dotted(&self, dotted: &str) -> bool {
        if dotted.is_empty() {
            return false;
        }
        // Fast read path
        if let Some(found) = self.cache.read().ok().and_then(|c| c.get(dotted).copied()) {
            return found;
        }
        // Resolve and cache
        let is_local = self.exists_in_root(dotted);
        if let Ok(mut c) = self.cache.write() {
            c.insert(dotted.to_string(), is_local);
        }
        is_local
    }

    /// Returns true if the module exists under root.
    fn exists_in_root(&self, dotted: &str) -> bool {
        if let Some(root_mod) = &self.root_module {
            if dotted == root_mod {
                return self.root_dir.join("__init__.py").exists();
            }
            if let Some(stripped) = dotted.strip_prefix(&(root_mod.clone() + ".")) {
                let rel = stripped.replace('.', "/");
                let file = self.root_dir.join(format!("{}.py", rel));
                if file.exists() {
                    return true;
                }
                return self.root_dir.join(&rel).join("__init__.py").exists();
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

    /// Classify a module as local or external, with a human-readable reason for external.
    pub fn classify_module(&self, module: &ModulePath) -> (bool, String) {
        let dotted = module.to_dotted();
        if self.is_local_dotted(&dotted) {
            return (true, String::new());
        }

        // Compute why it's considered external
        if let Some(root_mod) = &self.root_module {
            if !(dotted == *root_mod || dotted.starts_with(&(root_mod.clone() + "."))) {
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

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::module_path::ModulePath;

#[derive(Clone, Default)]
pub struct ImportResolver {
    cache: Arc<RwLock<HashMap<String, bool>>>,
    root: PathBuf,
}

impl ImportResolver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            root: root.into(),
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
        let rel = dotted.replace('.', "/");
        let file = self.root.join(format!("{}.py", rel));
        if file.exists() {
            return true;
        }
        let pkg_init = self.root.join(&rel).join("__init__.py");
        pkg_init.exists()
    }

    /// Helper for ModulePath input.
    pub fn is_local_module(&self, module: &ModulePath) -> bool {
        self.is_local_dotted(&module.to_dotted())
    }
}

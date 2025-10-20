use std::collections::HashMap;
use std::path::Path;

use crate::imports::import_line::ImportLine;

use super::ImportRule;

/// Linear order inside a configured source folder.
/// Uses a configured ordered list of submodules to constrain import order.
pub struct LinearOrderInFolder {
    source_folder: Vec<String>,
    order_index: HashMap<String, usize>,
    verbose: bool,
}

impl LinearOrderInFolder {
    pub fn new(source_folder: Vec<String>, order: Vec<String>, verbose: bool) -> Self {
        let mut order_index = HashMap::new();
        for (idx, name) in order.iter().enumerate() {
            order_index.insert(name.clone(), idx);
        }
        LinearOrderInFolder {
            source_folder,
            order_index,
            verbose,
        }
    }
}

impl ImportRule for LinearOrderInFolder {
    fn name(&self) -> &'static str {
        "linear_order_in_folder"
    }

    fn check_line(&self, current_file: &Path, import: &ImportLine) -> bool {
        // Only apply to files under source_folder module (match against path segments)
        let applies = current_file
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<String>>()
            .windows(self.source_folder.len())
            .any(|w| {
                w.iter()
                    .map(|s| s.as_str())
                    .eq(self.source_folder.iter().map(|s| s.as_str()))
            });
        if self.verbose {
            println!(
                "[linear] file={} applies={} src={:?}",
                current_file.to_string_lossy(),
                applies,
                self.source_folder
            );
        }
        if !applies {
            return true;
        }

        let current_module = current_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if current_module.is_empty() {
            if self.verbose {
                println!("[linear] skip: empty current_module");
            }
            return true;
        }

        let target_head = import.target_file.first().map(|s| s.as_str()).unwrap_or("");
        if target_head.is_empty() {
            if self.verbose {
                println!("[linear] skip: empty target_head");
            }
            return true;
        }
        let me_opt = self.order_index.get(current_module).copied();
        let other_opt = self.order_index.get(target_head).copied();
        let pass = match (me_opt, other_opt) {
            (Some(me), Some(other)) => other >= me,
            _ => true,
        };
        if self.verbose {
            println!(
                "[linear] cur={} tgt={} me_idx={:?} other_idx={:?} pass={}",
                current_module, target_head, me_opt, other_opt, pass
            );
        }
        pass
    }
}

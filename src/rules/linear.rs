use std::collections::HashMap;
use std::path::Path;

use crate::imports::import_line::ImportLine;
use crate::module_path::ModulePath;

use super::{ImportRule, RuleOutcome};

/// Linear order inside a configured source folder.
/// Uses a configured ordered list of submodules to constrain import order.
pub struct LinearOrderInFolder {
    source_folder: ModulePath,
    order_index: HashMap<String, usize>,
}

impl LinearOrderInFolder {
    pub fn new(source_folder: ModulePath, order: Vec<String>) -> Self {
        let mut order_index = HashMap::new();
        for (idx, name) in order.iter().enumerate() {
            order_index.insert(name.clone(), idx);
        }
        LinearOrderInFolder {
            source_folder,
            order_index,
        }
    }
}

impl ImportRule for LinearOrderInFolder {
    fn name(&self) -> &'static str {
        "Linear"
    }

    fn check_line(&self, _current_file: &Path, import: &ImportLine) -> RuleOutcome {
        // Only apply when the current module is under the configured source_folder
        let rel_from = match import.from_module.relative_from(&self.source_folder) {
            Some(mp) => mp,
            None => {
                return RuleOutcome {
                    pass: true,
                    reason: String::from("out of scope (not under source folder)"),
                }
            }
        };

        // Determine target head relative to the configured source_folder.
        let rel_target = match import.target_module.relative_from(&self.source_folder) {
            Some(mp) => mp,
            None => {
                return RuleOutcome {
                    pass: true,
                    reason: String::from("target not under source folder"),
                }
            }
        };
        let target_head = rel_target
            .segments()
            .first()
            .map(|s| s.as_str())
            .unwrap_or("");
        if target_head.is_empty() {
            return RuleOutcome {
                pass: true,
                reason: String::from("empty target head"),
            };
        }
        let current_head = rel_from
            .segments()
            .first()
            .map(|s| s.as_str())
            .unwrap_or("");
        if current_head.is_empty() {
            return RuleOutcome {
                pass: true,
                reason: String::from("empty current head"),
            };
        }
        let me_opt = self.order_index.get(current_head).copied();
        let other_opt = self.order_index.get(target_head).copied();
        let pass = match (me_opt, other_opt) {
            (Some(me), Some(other)) => other <= me,
            _ => true,
        };
        let reason = if pass {
            format!("ok: {} can import {}", current_head, target_head)
        } else {
            let mut ordered: Vec<(&String, &usize)> = self.order_index.iter().collect();
            ordered.sort_by_key(|(_, idx)| **idx);
            format!(
                "order violation: '{}' cannot import from '{}'",
                current_head, target_head
            )
        };
        RuleOutcome { pass, reason }
    }

    fn describe(&self) -> String {
        let folder = if self.source_folder.is_empty() {
            String::from("<unknown>")
        } else {
            self.source_folder.to_dotted()
        };
        let mut ordered: Vec<(&String, &usize)> = self.order_index.iter().collect();
        ordered.sort_by_key(|(_, idx)| **idx);
        let order = if ordered.is_empty() {
            String::from("<unspecified>")
        } else {
            ordered
                .into_iter()
                .map(|(k, _)| k.clone())
                .collect::<Vec<String>>()
                .join(" -> ")
        };
        format!("folder={} order={}", folder, order)
    }
}

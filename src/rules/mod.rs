use std::path::Path;

use crate::configs::{ProjectConfig, RunConfig};
use crate::imports::import_line::ImportLine;

pub mod linear;

#[derive(Debug, Clone)]
pub struct RuleOutcome {
    pub pass: bool,
    pub reason: String,
}

pub trait ImportRule {
    fn name(&self) -> &'static str;
    fn check_line(&self, current_file: &Path, import: &ImportLine) -> RuleOutcome;
    /// Human-readable summary of this rule's configuration for display.
    fn describe(&self) -> String;
}

pub fn build_rules(project: &ProjectConfig, config: &RunConfig) -> Vec<Box<dyn ImportRule>> {
    let mut rules: Vec<Box<dyn ImportRule>> = Vec::new();
    for linear in project.rules.linear.clone().into_iter() {
        let mut source_mp = linear.source_module.clone();
        if source_mp.is_empty() {
            source_mp = project.source_modules.first().cloned().unwrap_or_default();
        }

        // Validate configured source and ordered submodules exist
        let src_dir = source_mp.to_dir_pathbuf();
        let verbose = config.verbose.unwrap_or(false);
        if !src_dir.is_dir() {
            if verbose {
                eprintln!(
                    "[linear] warning: source module '{}' directory not found at {}",
                    source_mp.to_dotted(),
                    src_dir.to_string_lossy()
                );
            }
        } else {
            for elem in &linear.order {
                let sub_dir = src_dir.join(elem);
                let sub_file = src_dir.join(format!("{}.py", elem));
                if !sub_dir.is_dir() && !sub_file.is_file() {
                    if verbose {
                        eprintln!(
                            "[linear] warning: '{}' not found under '{}' (looked for {} or {})",
                            elem,
                            source_mp.to_dotted(),
                            sub_dir.to_string_lossy(),
                            sub_file.to_string_lossy()
                        );
                    }
                }
            }
        }

        rules.push(Box::new(crate::rules::linear::LinearOrderInFolder::new(
            source_mp,
            linear.order,
        )));
    }
    rules
}

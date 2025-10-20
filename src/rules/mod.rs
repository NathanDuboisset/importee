use std::path::Path;

use crate::configs::RunConfig;
use crate::imports::import_line::ImportLine;

pub mod linear;

pub trait ImportRule {
    fn name(&self) -> &'static str;
    fn check_line(&self, current_file: &Path, import: &ImportLine) -> bool;
}

pub fn build_rules(config: &RunConfig) -> Vec<Box<dyn ImportRule>> {
    let mut rules: Vec<Box<dyn ImportRule>> = Vec::new();
    if let Some(r) = &config.rules {
        if let Some(linear) = r.linear.clone() {
            rules.push(Box::new(crate::rules::linear::LinearOrderInFolder::new(
                config.source_module.clone(),
                linear.order,
                config.verbose.unwrap_or(false),
            )));
        }
    }
    rules
}

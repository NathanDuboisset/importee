use crate::module_path::ModulePath;
use std::fmt;

#[derive(Debug)]
pub struct ImportLine {
    pub from_module: ModulePath,
    pub target_module: ModulePath,
    pub import_line: i32,
}

impl fmt::Display for ImportLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let from = if self.from_module.is_empty() {
            String::from("<unknown>")
        } else {
            self.from_module.to_dotted()
        };
        let target = if self.target_module.is_empty() {
            String::from("<unknown>")
        } else {
            self.target_module.to_dotted()
        };
        write!(f, "line {}: {} -> {}", self.import_line, from, target)
    }
}

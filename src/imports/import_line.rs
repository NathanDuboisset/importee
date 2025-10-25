use crate::module_path::ModulePath;
use rustpython_ast::{Mod, Stmt};
use rustpython_parser::{parse, Mode};
use std::fmt;
use std::fs;

#[derive(Debug)]
pub struct ImportLine {
    pub from_module: ModulePath,
    pub target_module: ModulePath,
    pub import_line: i32,
}

// AST-based import collection

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

/// Parse imports for a module identified by its ModulePath. This preserves the full dotted path
/// for `from_module` instead of only using the file's stem.
pub fn get_file_imports_for_module(module: &ModulePath) -> Vec<ImportLine> {
    let file_path = module.file_path();
    let file_content = match fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    let ast = match parse(&file_content, Mode::Module, &file_path.to_string_lossy()) {
        Ok(suite) => suite,
        Err(_) => return Vec::new(),
    };

    let mut results: Vec<ImportLine> = Vec::new();

    let body: &[Stmt] = match &ast {
        Mod::Module(m) => &m.body,
        _ => &[],
    };

    for stmt in body.iter() {
        collect_imports_from_stmt(stmt, module, &mut results);
    }

    results
}

fn collect_imports_from_stmt(stmt: &Stmt, current_module: &ModulePath, out: &mut Vec<ImportLine>) {
    match stmt {
        Stmt::Import(inner) => {
            let line_no: i32 = 0;
            for alias in &inner.names {
                let target_module = ModulePath::from_dotted(&alias.name.to_string());
                out.push(ImportLine {
                    from_module: current_module.clone(),
                    target_module,
                    import_line: line_no,
                });
            }
        }
        Stmt::ImportFrom(inner) => {
            // Maintain previous behavior: one entry per 'from ... import ...', targeting the module only
            let level: usize = if inner.level.is_some() { 1 } else { 0 };
            let dots = ".".repeat(level);
            let module_name = inner
                .module
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_else(String::new);
            let spec = format!("{}{}", dots, module_name);
            let target_module = if spec.is_empty() {
                ModulePath::default()
            } else {
                ModulePath::from_import(current_module, &spec)
            };
            let line_no: i32 = 0;
            out.push(ImportLine {
                from_module: current_module.clone(),
                target_module,
                import_line: line_no,
            });
        }
        _ => {}
    }
}

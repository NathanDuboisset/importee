use crate::configs::RunConfig;
use crate::imports::classification::ImportResolver;
use crate::module_path::ModulePath;
use rustpython_ast::{Mod, Ranged, Stmt};
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
pub fn get_file_imports(
    module: &ModulePath,
    resolver: &ImportResolver,
    run_config: &RunConfig,
) -> Vec<ImportLine> {
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
        collect_imports_deep(
            stmt,
            module,
            resolver,
            &file_content,
            &mut results,
            run_config,
        );
    }

    results
}

fn collect_imports_from_stmt(
    stmt: &Stmt,
    current_module: &ModulePath,
    resolver: &ImportResolver,
    source: &str,
    out: &mut Vec<ImportLine>,
    run_config: &RunConfig,
) {
    let mut base: Option<String> = None;
    let mut line_no: i32 = 0;

    match stmt {
        Stmt::Import(inner) => {
            let start = inner.range().start().to_usize();
            line_no = (1 + source[..start].bytes().filter(|&b| b == b'\n').count()) as i32;
            if let Some(alias) = inner.names.first() {
                base = Some(alias.name.to_string());
            }
        }
        Stmt::ImportFrom(inner) => {
            let start = inner.range().start().to_usize();
            line_no = (1 + source[..start].bytes().filter(|&b| b == b'\n').count()) as i32;
            // Prefer the module; only use relative dots when module is missing
            let module_name = inner
                .module
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_default();
            if !module_name.is_empty() {
                // If first alias is a submodule that exists, prefer pkg.alias; else pkg
                if let Some(first) = inner.names.first() {
                    let try_sub = format!("{}.{}", module_name, first.name);
                    let resolved_try = resolver.resolve_import(current_module, &try_sub);
                    if resolver.is_local_module(&resolved_try) {
                        base = Some(try_sub);
                    } else {
                        base = Some(module_name);
                    }
                } else {
                    base = Some(module_name);
                }
            } else if let Some(first) = inner.names.first() {
                let dots = if inner.level.is_some() {
                    String::from(".")
                } else {
                    String::new()
                };
                base = Some(format!("{}{}", dots, first.name));
            }
        }
        _ => {}
    }

    if let Some(base_spec) = base {
        if run_config.verbose.unwrap_or(false) {
            println!(
                "[collect] from={} base={}",
                current_module.to_dotted(),
                base_spec
            );
        }
        let resolved = resolver.resolve_import(current_module, &base_spec);
        if resolver.is_local_module(&resolved) {
            out.push(ImportLine {
                from_module: current_module.clone(),
                target_module: resolved,
                import_line: line_no,
            });
        }
    }
}

fn collect_imports_deep(
    stmt: &Stmt,
    current_module: &ModulePath,
    resolver: &ImportResolver,
    source: &str,
    out: &mut Vec<ImportLine>,
    run_config: &RunConfig,
) {
    collect_imports_from_stmt(stmt, current_module, resolver, source, out, run_config);
    match stmt {
        Stmt::FunctionDef(inner) => {
            for s in inner.body.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
        }
        Stmt::ClassDef(inner) => {
            for s in inner.body.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
        }
        Stmt::If(inner) => {
            for s in inner.body.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
            for s in inner.orelse.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
        }
        Stmt::With(inner) => {
            for s in inner.body.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
        }
        Stmt::For(inner) => {
            for s in inner.body.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
            for s in inner.orelse.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
        }
        Stmt::While(inner) => {
            for s in inner.body.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
            for s in inner.orelse.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
        }
        Stmt::Try(inner) => {
            for s in inner.body.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
            for s in inner.orelse.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
            for s in inner.finalbody.iter() {
                collect_imports_deep(s, current_module, resolver, source, out, run_config);
            }
        }
        _ => {}
    }
}

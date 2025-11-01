use crate::configs::RunConfig;
use crate::imports::classification::ImportResolver;
use crate::imports::import_line::ImportLine;
use crate::module_path::ModulePath;
use rustpython_ast::{Mod, Ranged, Stmt};
use rustpython_parser::{parse, Mode};
use std::fs;

/// Build a line offset table for fast line number lookups.
/// Returns a vector where offsets[i] is the byte offset of line i+1.
fn build_line_offsets(source: &str) -> Vec<usize> {
    let mut offsets = vec![0];
    for (i, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            offsets.push(i + 1);
        }
    }
    offsets
}

/// Convert a byte offset to a line number using the pre-built offset table.
/// Binary search for O(log n) lookup instead of O(n) counting.
fn offset_to_line(offset: usize, line_offsets: &[usize]) -> u32 {
    match line_offsets.binary_search(&offset) {
        Ok(line) => (line + 1) as u32,
        Err(line) => line as u32,
    }
}

/// Parse imports for a module identified by its ModulePath. This preserves the full dotted path
/// for `from_module` instead of only using the file's stem.
/// If file_content is provided, it will be used instead of reading the file (performance optimization).
pub fn get_file_imports(
    module: &ModulePath,
    resolver: &ImportResolver,
    run_config: &RunConfig,
    file_content: Option<&str>,
) -> Vec<ImportLine> {
    let file_path = module.file_path();
    let content: String;
    let file_content_ref = match file_content {
        Some(c) => c,
        None => {
            content = match fs::read_to_string(&file_path) {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };
            &content
        }
    };

    // Parse with rustpython parser
    let ast = match parse(file_content_ref, Mode::Module, &file_path.to_string_lossy()) {
        Ok(suite) => suite,
        Err(_) => return Vec::new(),
    };

    // Build line offset table once for O(log n) line number lookups
    let line_offsets = build_line_offsets(file_content_ref);

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
            file_content_ref,
            &line_offsets,
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
    line_offsets: &[usize],
    out: &mut Vec<ImportLine>,
    run_config: &RunConfig,
) {
    let mut base: Option<String> = None;
    let mut line_no: u32 = 0;

    match stmt {
        Stmt::Import(inner) => {
            let start = inner.range().start().to_usize();
            line_no = offset_to_line(start, line_offsets);
            if let Some(alias) = inner.names.first() {
                base = Some(alias.name.to_string());
            }
        }
        Stmt::ImportFrom(inner) => {
            let start = inner.range().start().to_usize();
            line_no = offset_to_line(start, line_offsets);
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
    line_offsets: &[usize],
    out: &mut Vec<ImportLine>,
    run_config: &RunConfig,
) {
    collect_imports_from_stmt(
        stmt,
        current_module,
        resolver,
        line_offsets,
        out,
        run_config,
    );

    // PERFORMANCE: Deep traversal disabled - only collect top-level imports
    // Uncomment below to re-enable collecting imports from inside functions, classes, etc.
    /*
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
    */
}

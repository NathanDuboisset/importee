use pyo3::prelude::*;
// use serde::{Deserialize, Serialize};

mod configs;
mod imports;
mod module_path;
mod results;
mod rules;

use crate::imports::classification::ImportResolver;
use crate::imports::import_line::get_file_imports_for_module;
use crate::module_path::ModulePath;
use configs::{ProjectConfig, RunConfig};
use results::{CheckResult, Issue};
use std::fs;
use std::io::{self, Write};

fn run_check_imports(_project_config: ProjectConfig, run_config: RunConfig) -> CheckResult {
    // Debug: process only the configured source module file if provided
    let mut result = CheckResult::new();

    if !run_config.source_module.is_empty() {
        let tokens = run_config.source_module.clone();
        let module_path = ModulePath::new(tokens.clone());
        let dir_path = module_path.to_dir_pathbuf();
        if run_config.verbose.unwrap_or(false) {
            println!(
                "[core] source_module={:?} dir_path={}",
                tokens,
                dir_path.to_string_lossy()
            );
            println!("[core] active rules:");
            let rules = rules::build_rules(&run_config);
            for rule in rules.iter() {
                println!("  - {}: {}", rule.name(), rule.describe());
            }
        }
        if dir_path.is_dir() {
            if run_config.verbose.unwrap_or(false) {
                println!("[core] walking directory {}", dir_path.to_string_lossy());
            }
            let root_module = tokens.first().cloned();
            let resolver =
                ImportResolver::new(dir_path, root_module, run_config.verbose.unwrap_or(false));
            process_file_or_dir(&module_path, &run_config, &mut result, &resolver);
        } else if let Some((_leaf, _head)) = tokens.split_last() {
            let file_path = module_path.file_path();
            if run_config.verbose.unwrap_or(false) {
                println!(
                    "[core] dir missing; try file {} (parent={:?})",
                    file_path.to_string_lossy(),
                    _head
                );
            }
            let resolver = ImportResolver::new(
                file_path
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .to_path_buf(),
                tokens.first().cloned(),
                run_config.verbose.unwrap_or(false),
            );
            process_file_or_dir(&module_path, &run_config, &mut result, &resolver);
        } else if run_config.verbose.unwrap_or(false) {
            println!("[core] empty source_module tokens; nothing to do");
        }
    } else if let Ok(cwd) = std::env::current_dir() {
        if run_config.verbose.unwrap_or(false) {
            println!(
                "[core] no source_module provided; walking cwd {}",
                cwd.to_string_lossy()
            );
        }
        // Start walking from cwd as a ModulePath (empty path represents cwd root)
        let resolver = ImportResolver::new(&cwd, None, run_config.verbose.unwrap_or(false));
        walk_and_print_py_imports(
            &ModulePath::new(vec![]),
            &run_config,
            &mut result,
            &resolver,
        );
    }
    result
}

fn walk_and_print_py_imports(
    dir: &ModulePath,
    run_config: &RunConfig,
    result: &mut CheckResult,
    resolver: &ImportResolver,
) {
    let entries = match fs::read_dir(dir.to_dir_pathbuf()) {
        Ok(read_dir) => read_dir,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let file_name_os = entry.file_name();
        let file_name = file_name_os.to_string_lossy();
        let path = entry.path();

        // Skip Python cache directories explicitly
        if path.is_dir() && file_name == "__pycache__" {
            continue;
        }

        if path.is_dir() {
            let new_module_path = dir.append(file_name.to_string());
            walk_and_print_py_imports(&new_module_path, run_config, result, resolver);
        } else if path.is_file() {
            // Only process .py files; ignore .pyi, .pyc, .so, etc.
            if path.extension().and_then(|e| e.to_str()) != Some("py") {
                continue;
            }
            // Append stem (module name without extension) to ModulePath
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let new_module_path = dir.append(stem);
            process_file_or_dir(&new_module_path, run_config, result, resolver);
        }
    }
}

fn process_file_or_dir(
    module_path: &ModulePath,
    run_config: &RunConfig,
    result: &mut CheckResult,
    resolver: &ImportResolver,
) {
    if module_path.to_dir_pathbuf().is_dir() {
        if run_config.verbose.unwrap_or(false) {
            println!("[core] recurse into dir {}", module_path.to_dotted());
        }
        walk_and_print_py_imports(module_path, run_config, result, resolver);
        return;
    }
    // Always print file header in normal/verbose; quiet suppresses output
    if !run_config.quiet.unwrap_or(false) {
        println!("=== {} ===", module_path.file_path().to_string_lossy());
    }
    let _ = io::stdout().flush();
    let rules = rules::build_rules(run_config);
    let mut imports = Vec::new();
    for imp in get_file_imports_for_module(module_path, resolver, run_config).into_iter() {
        let (is_local, reason) = resolver.classify_module(&imp.target_module);
        if is_local {
            imports.push(imp);
        } else if run_config.verbose.unwrap_or(false) {
            println!(
                "[external] {} -> {} ({})",
                imp.from_module.to_dotted(),
                imp.target_module.to_dotted(),
                reason
            );
        }
    }
    // Print file visited for normal/verbose modes
    // Dotted name remains available via verbose/external output

    for imp in imports.iter() {
        if run_config.verbose.unwrap_or(false) {
            println!("{}", imp);
        }
        for rule in rules.iter() {
            let ok = rule.check_line(&module_path.file_path(), imp);
            if run_config.verbose.unwrap_or(false) {
                println!("  rule {}: {}", rule.name(), if ok { "ok" } else { "FAIL" });
            }
            if !ok {
                let message = if rule.name() == "linear_order_in_folder" {
                    let from = if imp.from_module.is_empty() {
                        String::from("<unknown>")
                    } else {
                        imp.from_module.to_dotted()
                    };
                    let target = if imp.target_module.is_empty() {
                        String::from("<unknown>")
                    } else {
                        imp.target_module.to_dotted()
                    };
                    let folder = if run_config.source_module.is_empty() {
                        String::from("<unknown>")
                    } else {
                        crate::module_path::ModulePath::new(run_config.source_module.clone())
                            .to_dotted()
                    };
                    let expected_order = if let Some(r) = &run_config.rules {
                        if let Some(linear) = &r.linear {
                            if linear.order.is_empty() {
                                String::from("<unspecified>")
                            } else {
                                linear.order.join(" -> ")
                            }
                        } else {
                            String::from("<unspecified>")
                        }
                    } else {
                        String::from("<unspecified>")
                    };
                    format!(
                        "linear_order_in_folder: line {}: import {} -> {} breaks order in module {}. Expected order: {}",
                        imp.import_line, from, target, folder, expected_order
                    )
                } else {
                    format!("rule '{}' failed at line {}", rule.name(), imp.import_line)
                };

                result.issues.push(Issue {
                    path: module_path.file_path().to_string_lossy().to_string(),
                    message,
                });
            }
        }
    }
    if imports.is_empty() && run_config.verbose.unwrap_or(false) {
        println!(
            "[core] no imports found in {}",
            module_path.file_path().to_string_lossy()
        );
    }
}

/// Run the importee checker, parse the project and run config and return the results as a string
#[pyfunction]
fn check_imports(project_config: String, run_config: String) -> PyResult<String> {
    let project_config: ProjectConfig = serde_json::from_str(&project_config).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("project_config json error: {}", e))
    })?;
    let run_config: RunConfig = serde_json::from_str(&run_config).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("run_config json error: {}", e))
    })?;

    let result = run_check_imports(project_config, run_config);
    let json = serde_json::to_string(&result).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("serialize error: {}", e))
    })?;
    Ok(json)
}

/// Python module definition
#[pymodule]
fn _rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_imports, m)?)?;
    Ok(())
}

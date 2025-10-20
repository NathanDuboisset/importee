use pyo3::prelude::*;
// use serde::{Deserialize, Serialize};

mod configs;
mod imports;
mod results;
mod rules;

use crate::imports::import_line::{get_file_imports_from_path, print_import_line};
use configs::{ProjectConfig, RunConfig};
use results::CheckResult;
use std::fs;
use std::path::Path;

fn run_check_imports(_project_config: ProjectConfig, run_config: RunConfig) -> CheckResult {
    // Debug: process only the configured source module file if provided
    if !run_config.source_module.is_empty() {
        let relative_path =
            crate::imports::base::get_relative_path(run_config.source_module.clone());
        let init_path_buf = format!("{}{}", &relative_path, "__init__.py");
        let file_path_buf = format!("{}{}.py", &relative_path, "");
        let init_path = Path::new(&init_path_buf);
        let file_path = Path::new(&file_path_buf);
        let chosen = if init_path.is_file() {
            init_path
        } else {
            file_path
        };
        if run_config.verbose.unwrap_or(false) {
            println!(
                "[core] source_module={:?} resolved init={} file={} chosen={}",
                run_config.source_module,
                init_path.to_string_lossy(),
                file_path.to_string_lossy(),
                chosen.to_string_lossy()
            );
        }
        process_file_or_dir(chosen, &run_config);
    } else if let Ok(cwd) = std::env::current_dir() {
        if run_config.verbose.unwrap_or(false) {
            println!(
                "[core] no source_module provided; walking cwd {}",
                cwd.to_string_lossy()
            );
        }
        walk_and_print_py_imports(&cwd, &run_config);
    }
    CheckResult::new()
}

fn walk_and_print_py_imports(dir: &Path, run_config: &RunConfig) {
    let entries = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_and_print_py_imports(&path, run_config);
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) == Some("py") {
            process_file_or_dir(&path, run_config);
        }
    }
}

fn process_file_or_dir(path: &Path, run_config: &RunConfig) {
    if path.is_dir() {
        if run_config.verbose.unwrap_or(false) {
            println!("[core] recurse into dir {}", path.to_string_lossy());
        }
        walk_and_print_py_imports(path, run_config);
        return;
    }
    let rules = rules::build_rules(run_config);
    let imports = get_file_imports_from_path(path);
    if !imports.is_empty() {
        if let Some(p) = path.to_str() {
            println!("=== {} ===", p);
        }
        for imp in imports.iter() {
            print_import_line(imp);
            for rule in rules.iter() {
                let ok = rule.check_line(path, imp);
                println!("  rule {}: {}", rule.name(), if ok { "ok" } else { "FAIL" });
            }
        }
    } else if run_config.verbose.unwrap_or(false) {
        println!("[core] no imports found in {}", path.to_string_lossy());
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

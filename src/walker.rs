use crate::configs::{ProjectConfig, RunConfig};
use crate::imports::classification::ImportResolver;
use crate::module_path::ModulePath;
use crate::results::CheckResult;
use std::fs;

use crate::file_processor::process_file;

pub fn run_check_imports(project_config: ProjectConfig, run_config: RunConfig) -> CheckResult {
    let mut result = CheckResult::new();
    let mut walked_any = false;
    if !project_config.source_modules.is_empty() {
        for module_path in project_config.source_modules.iter().cloned() {
            let dir_path = module_path.to_dir_pathbuf();
            if run_config.verbose.unwrap_or(false) {
                println!(
                    "[core] source_module={} dir_path={}",
                    module_path.to_dotted(),
                    dir_path.to_string_lossy()
                );
                println!("[core] active rules:");
                let rules = crate::rules::build_rules(&project_config, &run_config);
                for rule in rules.iter() {
                    println!("  - {}: {}", rule.name(), rule.describe());
                }
            }
            if dir_path.is_dir() {
                if run_config.verbose.unwrap_or(false) {
                    println!("[core] walking directory {}", dir_path.to_string_lossy());
                }
                let root_module = module_path.segments().first().cloned();
                let resolver =
                    ImportResolver::new(dir_path, root_module, run_config.verbose.unwrap_or(false));
                walk_dirs(
                    &module_path,
                    &project_config,
                    &run_config,
                    &mut result,
                    &resolver,
                );
                walked_any = true;
            } else if let Some((_leaf, _head)) = module_path.split_last() {
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
                    module_path.segments().first().cloned(),
                    run_config.verbose.unwrap_or(false),
                );
                process_file(
                    &module_path,
                    &project_config,
                    &run_config,
                    &mut result,
                    &resolver,
                );
                walked_any = true;
            } else if run_config.verbose.unwrap_or(false) {
                println!("[core] empty source_module tokens; nothing to do");
            }
        }
    }
    if !walked_any {
        if let Ok(cwd) = std::env::current_dir() {
            if run_config.verbose.unwrap_or(false) {
                println!(
                    "[core] no source_module provided; walking cwd {}",
                    cwd.to_string_lossy()
                );
            }
            // Start walking from cwd as a ModulePath (empty path represents cwd root)
            let resolver = ImportResolver::new(&cwd, None, run_config.verbose.unwrap_or(false));
            println!("[core] active rules:");
            let rules = crate::rules::build_rules(&project_config, &run_config);
            for rule in rules.iter() {
                println!("  - {}: {}", rule.name(), rule.describe());
            }
            walk_dirs(
                &ModulePath::new(vec![]),
                &project_config,
                &run_config,
                &mut result,
                &resolver,
            );
        }
    }
    result
}

pub fn walk_dirs(
    dir: &ModulePath,
    project_config: &ProjectConfig,
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
            walk_dirs(
                &new_module_path,
                project_config,
                run_config,
                result,
                resolver,
            );
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
            process_file(
                &new_module_path,
                project_config,
                run_config,
                result,
                resolver,
            );
        }
    }
}

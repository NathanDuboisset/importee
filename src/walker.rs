use crate::configs::{ProjectConfig, RunConfig};
use crate::imports::classification::ImportResolver;
use crate::module_path::ModulePath;
use crate::results::{CheckResult, Issue};
use crate::rules::ImportRule;
use rayon::prelude::*;
use std::fs;

pub fn run_check_imports(project_config: ProjectConfig, run_config: RunConfig) -> CheckResult {
    let mut result = CheckResult::new();

    // Determine sources: use source_modules or fallback to cwd
    let sources: Vec<ModulePath> = if !project_config.source_modules.is_empty() {
        project_config.source_modules.clone()
    } else {
        vec![ModulePath::new(vec![])] // empty path represents cwd root
    };

    // OPTIMIZATION: Build rules once at the top level instead of per-file
    let rules = crate::rules::build_rules(&project_config, &run_config);

    // Print active rules once if verbose
    if run_config.verbose.unwrap_or(false) {
        println!("[core] active rules:");
        for rule in rules.iter() {
            println!("  - {}: {}", rule.name(), rule.describe());
        }
    }

    // Walk each source in parallel
    let all_issues: Vec<Issue> = sources
        .par_iter()
        .flat_map(|module_path| {
            if run_config.verbose.unwrap_or(false) {
                println!(
                    "[core] walking {} ({})",
                    module_path.to_dotted(),
                    module_path.to_dir_pathbuf().to_string_lossy()
                );
            }

            let root_module = module_path.segments().first().cloned();
            let root_dir = if module_path.to_dir_pathbuf().is_dir() {
                module_path.to_dir_pathbuf()
            } else {
                module_path
                    .file_path()
                    .parent()
                    .unwrap_or_else(|| std::path::Path::new("."))
                    .to_path_buf()
            };
            let resolver =
                ImportResolver::new(root_dir, root_module, run_config.verbose.unwrap_or(false));

            walk_path_parallel(module_path, &run_config, &resolver, &rules)
        })
        .collect();

    result.issues.extend(all_issues);
    result
}

/// Walk a path (file or directory) and process it in parallel
/// Rules are filtered at each level based on check_concern to avoid unnecessary checks
fn walk_path_parallel(
    path: &ModulePath,
    run_config: &RunConfig,
    resolver: &ImportResolver,
    rules: &[Box<dyn ImportRule>],
) -> Vec<Issue> {
    // OPTIMIZATION: Filter rules that are concerned with this path
    let verbose = run_config.verbose.unwrap_or(false);
    let relevant_rules: Vec<&Box<dyn ImportRule>> = rules
        .iter()
        .filter(|rule| rule.check_concern(path, verbose))
        .collect();

    // OPTIMIZATION: If no rules apply to this path, skip entirely
    if relevant_rules.is_empty() {
        if verbose {
            println!("[walker] skipping {} - no rules apply", path.to_dotted());
        }
        return Vec::new();
    }

    let target = path.to_dir_pathbuf();

    // If it's a directory, walk it recursively
    if target.is_dir() {
        let entries = match fs::read_dir(&target) {
            Ok(read_dir) => read_dir,
            Err(_) => return Vec::new(),
        };

        // Collect entries to process
        let entries: Vec<_> = entries.flatten().collect();

        // Process all entries in parallel
        entries
            .par_iter()
            .flat_map(|entry| {
                let file_name_os = entry.file_name();
                let file_name = file_name_os.to_string_lossy();
                let entry_path = entry.path();

                // Skip Python cache directories explicitly
                if entry_path.is_dir() && file_name == "__pycache__" {
                    return Vec::new();
                }

                if entry_path.is_dir() {
                    let new_module_path = path.append(file_name.to_string());
                    // Recursively walk subdirectory - rules will be filtered again
                    walk_path_parallel(&new_module_path, run_config, resolver, rules)
                } else if entry_path.is_file() {
                    // Only process .py files; ignore .pyi, .pyc, .so, etc.
                    if entry_path.extension().and_then(|e| e.to_str()) != Some("py") {
                        return Vec::new();
                    }
                    // Append stem (module name without extension) to ModulePath
                    let stem = match entry_path.file_stem().and_then(|s| s.to_str()) {
                        Some(s) => s.to_string(),
                        None => return Vec::new(),
                    };
                    let new_module_path = path.append(stem);

                    // Process file with only the relevant rules
                    crate::file_processor::process_file_with_rules(
                        &new_module_path,
                        run_config,
                        resolver,
                        &relevant_rules,
                    )
                } else {
                    Vec::new()
                }
            })
            .collect()
    } else if target.is_file() || path.file_path().is_file() {
        // It's a single file - process it directly with relevant rules
        crate::file_processor::process_file_with_rules(path, run_config, resolver, &relevant_rules)
    } else {
        Vec::new()
    }
}

use crate::configs::RunConfig;
use crate::imports::classification::ImportResolver;
use crate::imports::collection::get_file_imports;
use crate::module_path::ModulePath;
use crate::results::Issue;
use crate::rules::ImportRule;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    #[serde(default = "cache_version_current")]
    version: u8,
    hash: String,
    // (target_dotted, line_no)
    imports: Vec<(String, i32)>,
}

fn cache_version_current() -> u8 {
    2
}

/// Compute hash from file content string (avoids re-reading the file)
fn compute_hash_from_string(content: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(content.as_bytes());
    hasher.finalize().to_hex().to_string()
}

fn find_project_root(start: &Path) -> PathBuf {
    let mut cur = start;
    // Walk up until we find a pyproject.toml, else fallback to start
    loop {
        let candidate = cur.join("pyproject.toml");
        if candidate.exists() {
            return cur.to_path_buf();
        }
        if let Some(parent) = cur.parent() {
            cur = parent;
        } else {
            return start.to_path_buf();
        }
    }
}

fn cache_file_path(resolver: &ImportResolver, module_path: &ModulePath) -> PathBuf {
    let project_root = find_project_root(resolver.root_dir());
    let cache_root = project_root.join(".importee_cache");
    let rel_file = module_path.file_path();
    let mut cache_path = cache_root.join(rel_file);
    cache_path.set_extension("imports.json");
    cache_path
}

fn try_load_cache(
    resolver: &ImportResolver,
    module_path: &ModulePath,
    hash: &str,
) -> Option<Vec<crate::imports::import_line::ImportLine>> {
    let path = cache_file_path(resolver, module_path);
    let data = fs::read_to_string(path).ok()?;
    let entry: CacheEntry = serde_json::from_str(&data).ok()?;
    // Invalidate old cache formats (without line numbers)
    if entry.version < 2 {
        return None;
    }
    if entry.hash != hash {
        return None;
    }
    let mut out = Vec::with_capacity(entry.imports.len());
    for (target_dotted, line) in entry.imports.into_iter() {
        out.push(crate::imports::import_line::ImportLine {
            from_module: module_path.clone(),
            target_module: ModulePath::from_dotted(&target_dotted),
            import_line: line,
        });
    }
    Some(out)
}

fn ensure_cache_dir(cache_root: &Path) {
    // Create cache directory if it doesn't exist
    let _ = fs::create_dir_all(cache_root);

    // Ensure .gitignore exists with * to ignore all cache files
    let gitignore_path = cache_root.join(".gitignore");
    if !gitignore_path.exists() {
        let _ = fs::write(gitignore_path, "*\n");
    }
}

fn save_cache(
    resolver: &ImportResolver,
    module_path: &ModulePath,
    hash: &str,
    imports: &[crate::imports::import_line::ImportLine],
) {
    let path = cache_file_path(resolver, module_path);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Ensure cache root directory has .gitignore
    let project_root = find_project_root(resolver.root_dir());
    let cache_root = project_root.join(".importee_cache");
    ensure_cache_dir(&cache_root);
    let flat: Vec<(String, i32)> = imports
        .iter()
        .map(|imp| (imp.target_module.to_dotted(), imp.import_line))
        .collect();
    let entry = CacheEntry {
        version: cache_version_current(),
        hash: hash.to_string(),
        imports: flat,
    };
    if let Ok(json) = serde_json::to_string(&entry) {
        let _ = fs::write(path, json);
    }
}

/// OPTIMIZED: Process a file with pre-built rules (avoids rebuilding rules per file)
/// Returns a Vec<Issue> instead of mutating a CheckResult
pub fn process_file_with_rules(
    module_path: &ModulePath,
    run_config: &RunConfig,
    resolver: &ImportResolver,
    rules: &[&Box<dyn ImportRule>],
) -> Vec<Issue> {
    // Only handle files here; directory walking is managed by walker
    if module_path.to_dir_pathbuf().is_dir() {
        return Vec::new();
    }

    // Always print file header in verbose; quiet suppresses output
    if run_config.verbose.unwrap_or(false) {
        println!("=== {} ===", module_path.file_path().to_string_lossy());
    }
    let _ = io::stdout().flush();

    // Read file once and compute hash from content (avoid double read)
    let file_path = module_path.file_path();
    let file_content = match fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(), // Can't read file, skip it
    };
    let file_hash = compute_hash_from_string(&file_content);

    let disable_cache = run_config.no_cache.unwrap_or(false);
    let mut imports = if disable_cache {
        Vec::new()
    } else {
        if let Some(cached) = try_load_cache(resolver, module_path, &file_hash) {
            cached
        } else {
            Vec::new()
        }
    };

    if imports.is_empty() {
        // Pass the file content we already read to avoid re-reading
        for imp in
            get_file_imports(module_path, resolver, run_config, Some(&file_content)).into_iter()
        {
            imports.push(imp);
        }
        if !disable_cache {
            save_cache(resolver, module_path, &file_hash, &imports);
        }
    }

    for imp in imports.iter() {
        let (is_local, reason) = resolver.classify_module(&imp.target_module);
        if is_local {
            // keep
        } else if run_config.verbose.unwrap_or(false) {
            println!(
                "[external] {} -> {} ({})",
                imp.from_module.to_dotted(),
                imp.target_module.to_dotted(),
                reason
            );
        }
    }

    let mut issues = Vec::new();

    for imp in imports.iter() {
        if run_config.verbose.unwrap_or(false) {
            println!("{}", imp);
        }
        for rule in rules.iter() {
            let outcome = rule.check_line(&module_path.file_path(), imp);
            if run_config.verbose.unwrap_or(false) && !outcome.pass {
                println!(
                    "[{}] imported \"{}\" : {}",
                    rule.name(),
                    imp.target_module.to_dotted(),
                    outcome.reason
                );
            }
            if !outcome.pass {
                let message = format!(
                    "imported \"{}\" : {}",
                    imp.target_module.to_dotted(),
                    outcome.reason
                );
                issues.push(Issue {
                    rule_name: rule.name().to_string(),
                    path: module_path.file_path().to_string_lossy().to_string(),
                    line: imp.import_line,
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

    issues
}

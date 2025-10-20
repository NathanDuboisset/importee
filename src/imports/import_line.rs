use crate::imports::base::get_relative_path;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fs;
use std::path::Path;

#[derive(Debug)]
pub struct ImportLine {
    pub from_file: Vec<String>,
    pub target_file: Vec<String>,
    pub import_line: i32,
}

static RE_FROM: Lazy<Regex> = Lazy::new(|| {
    // Matches: from pkg.sub import a, b as c
    // Captures module in group 1 and imported list in group 2
    Regex::new(r"^from\s+([^\s]+)\s+import\s+(.+)$").unwrap()
});

static RE_IMPORT: Lazy<Regex> = Lazy::new(|| {
    // Matches: import a, b as c
    // Captures imported list in group 1
    Regex::new(r"^import\s+(.+)$").unwrap()
});

pub fn print_import_line(line: &ImportLine) {
    let from = if line.from_file.is_empty() {
        String::from("<unknown>")
    } else {
        line.from_file.join(".")
    };
    let target = if line.target_file.is_empty() {
        String::from("<unknown>")
    } else {
        line.target_file.join(".")
    };
    println!("line {}: {} -> {}", line.import_line, from, target);
}

pub fn get_file_imports_from_path(file_path: &Path) -> Vec<ImportLine> {
    let file_content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(_) => String::new(),
    };

    let mut results: Vec<ImportLine> = Vec::new();

    // Determine current module name (file stem without extension)
    let current_module: String = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    for (idx, raw_line) in file_content.lines().enumerate() {
        let line_no_comments = raw_line.split('#').next().unwrap_or("");
        let line = line_no_comments.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(caps) = RE_FROM.captures(line) {
            let module = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            // New semantics: target is the module path being imported from (split by '.')
            let target_tokens: Vec<String> = if module.is_empty() {
                Vec::new()
            } else {
                module
                    .split('.')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            };

            results.push(ImportLine {
                from_file: if current_module.is_empty() {
                    Vec::new()
                } else {
                    vec![current_module.clone()]
                },
                target_file: target_tokens,
                import_line: (idx + 1) as i32,
            });
        } else if let Some(caps) = RE_IMPORT.captures(line) {
            let list = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            for part in list.split(',') {
                let name = part.trim();
                if name.is_empty() {
                    continue;
                }
                // Drop alias after "as"
                let tokens_ws = name.split_whitespace().collect::<Vec<_>>();
                let cleaned = if let Some(pos) = tokens_ws.iter().position(|&t| t == "as") {
                    tokens_ws[..pos].join(" ")
                } else {
                    name.to_string()
                };
                if cleaned.is_empty() {
                    continue;
                }

                let target_tokens: Vec<String> = cleaned
                    .split('.')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                results.push(ImportLine {
                    from_file: if current_module.is_empty() {
                        Vec::new()
                    } else {
                        vec![current_module.clone()]
                    },
                    target_file: target_tokens,
                    import_line: (idx + 1) as i32,
                });
            }
        }
    }

    results
}

pub fn get_file_imports(module_path: Vec<String>) -> Vec<ImportLine> {
    let relative_path = get_relative_path(module_path);
    let file_path = Path::new(&relative_path);
    get_file_imports_from_path(file_path)
}

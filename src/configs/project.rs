#[derive(Deserialize, Debug, Clone, Default)]
pub struct LinearRuleDef {
    pub order: Vec<String>,
    #[serde(default)]
    pub source_module: ModulePath,
}
use crate::module_path::ModulePath;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ProjectConfig {
    /// List of source modules
    pub source_modules: Vec<ModulePath>,
    /// Project-scoped rules configuration
    #[serde(default)]
    pub rules: ProjectRulesConfig,
    /// List of glob patterns to exclude from checking
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct ProjectRulesConfig {
    /// Multiple linear rules supported
    #[serde(default)]
    pub linear: Vec<LinearRuleDef>,
}

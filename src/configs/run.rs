use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct LinearRuleConfig {
    pub order: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct RunConfig {
    pub source_module: Vec<String>,
    pub rules: Option<RulesConfig>,
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct RulesConfig {
    pub linear: Option<LinearRuleConfig>,
}

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default)]
pub struct RunConfig {
    pub verbose: Option<bool>,
    pub quiet: Option<bool>,
    pub no_cache: Option<bool>,
}

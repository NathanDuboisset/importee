use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub classifiers: Vec<String>,
    pub dependencies: Vec<String>,
}

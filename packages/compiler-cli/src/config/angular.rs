use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct AngularConfig {
    pub projects: HashMap<String, Project>,
    #[serde(rename = "defaultProject")]
    pub default_project: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Project {
    #[serde(rename = "sourceRoot")]
    pub source_root: Option<String>,
    pub architect: Option<HashMap<String, ArchitectTarget>>,
}

#[derive(Debug, Deserialize)]
pub struct ArchitectTarget {
    pub builder: String,
    pub options: Option<BuildOptions>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildOptions {
    pub main: Option<String>,
    #[serde(rename = "tsConfig")]
    pub ts_config: Option<String>,
    pub output_path: Option<String>,
    pub index: Option<String>,
    pub polyfills: Option<Vec<String>>,
    pub assets: Option<Vec<Asset>>,
    pub styles: Option<Vec<String>>,
    pub scripts: Option<Vec<String>>,
    pub aot: Option<bool>,
    pub source_map: Option<bool>,
    pub optimization: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Asset {
    String(String),
    Object {
        glob: String,
        input: String,
        output: Option<String>,
    },
}

impl AngularConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: AngularConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn get_project(&self, name: Option<&str>) -> Option<(&String, &Project)> {
        if let Some(name) = name {
            self.projects.get_key_value(name)
        } else if let Some(default) = &self.default_project {
            self.projects.get_key_value(default)
        } else {
            self.projects.iter().next()
        }
    }
}

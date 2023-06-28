use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::{
  fs,
  path::PathBuf,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
  pub witness_dir: PathBuf,
  pub logs_dir: PathBuf,
}

impl Default for Config {
  fn default() -> Self {
    Config {
      witness_dir: PathBuf::from(r"C:\Program Files\Steam\steamapps\common\The Witness\"),
      logs_dir: std::env::current_dir().unwrap(),
    }
  }
}

impl Config {
  pub fn get() -> Self {
    if let Ok(raw) = fs::read_to_string("config.json") {
      if let Ok(config) = serde_json::from_str(&raw) {
        config
      } else {
          Config::default()
      }
    } else {
      Config::default()
    }
  }

  pub fn save(&self) -> Result<()> {
    let json = serde_json::to_string_pretty(self)?;
    std::fs::write("config.json", json)?;
    Ok(())
  }
}

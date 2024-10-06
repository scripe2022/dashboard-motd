// run  := cargo run --
// dir  := .
// kid  :=

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

fn default_true() -> bool { true }

fn default_none() -> String { "none".to_string() }

#[derive(Deserialize, Debug)]
pub struct SysService {
    #[serde(default = "default_none")]
    pub name:    String,
    #[serde(default = "default_none")]
    pub display: String
}

#[derive(Deserialize)]
pub struct SysDocker {
    #[serde(default = "default_none")]
    pub name:    String,
    #[serde(default = "default_none")]
    pub display: String
}

#[derive(Deserialize)]
pub struct SysDisk {
    #[serde(default = "default_none")]
    pub path:    String,
    #[serde(default = "default_none")]
    pub display: String,
    #[serde(default)]
    pub subvol:  Vec<String>
}

#[derive(Deserialize)]
pub struct SysVm {
    #[serde(default = "default_none")]
    pub name:    String,
    #[serde(default = "default_none")]
    pub display: String
}

#[derive(Deserialize)]
pub struct SysGpu {
    #[serde(default = "default_none")]
    pub command: String,
    #[serde(default = "default_none")]
    pub memdisplay: String,
    #[serde(default = "default_none")]
    pub tempdisplay: String
}

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_true")]
    pub memory: bool,

    #[serde(default = "default_true")]
    pub uptime: bool,

    #[serde(default = "default_true")]
    pub cpuload: bool,

    #[serde(default = "default_true")]
    pub lastlogin: bool,

    #[serde(default = "default_none")]
    pub cputemp: String,

    #[serde(default)]
    pub disk: Vec<SysDisk>,

    #[serde(default)]
    pub systemctl: Vec<SysService>,

    #[serde(default)]
    pub docker: Vec<SysDocker>,

    #[serde(default)]
    pub vm: Vec<SysVm>,

    #[serde(default)]
    pub gpu: Vec<SysGpu>
}

pub struct LoadConfig {
    config: Config
}

impl LoadConfig {
    pub fn new(config_path: Option<PathBuf>) -> Self {
        let content = if let Some(path) = config_path {
            if path.exists() {
                fs::read_to_string(&path).unwrap_or_else(|e| {
                    eprintln!("Failed to read config file: {}", e);
                    std::process::exit(1);
                })
            }
            else {
                eprintln!("Provided config file does not exist: {:?}", path);
                std::process::exit(1);
            }
        }
        else {
            let default_path = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("motd.toml");
            // println!("Using default config path: {:?}", default_path);
            if default_path.exists() {
                fs::read_to_string(&default_path).unwrap_or_else(|e| {
                    eprintln!("Failed to read default config file: {}", e);
                    String::new()
                })
            }
            else {
                String::new()
            }
        };

        let config: Config = toml::from_str(&content).unwrap_or_else(|e| {
            eprintln!("Failed to parse config file: {}", e);
            std::process::exit(1);
        });

        LoadConfig { config }
    }

    pub fn get_config(&self) -> &Config { &self.config }
}

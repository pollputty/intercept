use serde::Deserialize;
use std::{io::Result, path::PathBuf};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub log: LogConfig,
    pub record: RecordConfig,
    pub redirect: RedirectConfig,
}

#[derive(Debug, Deserialize)]
pub struct LogConfig {
    #[serde(default)]
    pub level: LogLevel,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    DEBUG,
    INFO,
    #[default]
    WARNING,
    ERROR,
}

#[derive(Debug, Deserialize)]
pub struct RedirectConfig {
    pub files: Vec<Redirect>,
    pub random: bool,
    pub time: Option<u64>,
    pub pid: Option<u32>,
    pub stdout: Option<PathBuf>,
    pub stderr: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecordConfig {
    pub files: bool,
    pub random: bool,
    pub time: bool,
    pub pid: bool,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Redirect {
    // pub redirect_type: RedirectType,
    pub from: String,
    pub to: String,
}

impl Config {
    pub fn load(filepath: &str) -> Result<Config> {
        let content = std::fs::read_to_string(filepath)?;
        let config = serde_yaml::from_str(content.as_str())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(config)
    }
}

impl From<&LogLevel> for tracing::Level {
    fn from(value: &LogLevel) -> Self {
        match value {
            LogLevel::DEBUG => tracing::Level::DEBUG,
            LogLevel::INFO => tracing::Level::INFO,
            LogLevel::WARNING => tracing::Level::WARN,
            LogLevel::ERROR => tracing::Level::ERROR,
        }
    }
}

pub struct SpawnOptions {
    pub stdout: Option<std::process::Stdio>,
    pub stderr: Option<std::process::Stdio>,
}

impl TryFrom<&Config> for SpawnOptions {
    type Error = std::io::Error;

    fn try_from(config: &Config) -> Result<Self> {
        let stdout = config.redirect.stdout.as_ref().map(|path| {
            Ok(std::process::Stdio::from(
                std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)?,
            ))
        });

        let stderr = config.redirect.stderr.as_ref().map(|path| {
            Ok(std::process::Stdio::from(
                std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(path)?,
            ))
        });

        // Cast Option<Result<Stdio>> to Option<Stdio>
        let stdout = match stdout {
            Some(Ok(stdio)) => Some(stdio),
            Some(Err(e)) => return Err(e),
            None => None,
        };
        let stderr = match stderr {
            Some(Ok(stdio)) => Some(stdio),
            Some(Err(e)) => return Err(e),
            None => None,
        };

        Ok(Self { stdout, stderr })
    }
}

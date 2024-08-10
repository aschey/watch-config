use std::io::{self};
use std::path::PathBuf;

use directories::ProjectDirs;
use schematic::Format;
use serde::{Deserialize, Serialize};
pub use watcher::*;

pub mod backend;

mod watcher;

pub trait LoadConfig {
    type Config: PartialEq;
    type Error;
    fn snapshot(&self) -> Self::Config;
    fn reload(&self) -> Result<Self::Config, Self::Error>;
    fn full_path(&self) -> PathBuf;
    fn edit(&self) -> Result<Self::Config, Self::Error> {
        let full_path = self.full_path();
        edit::edit_file(&full_path).unwrap();
        self.reload()
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Label {
    pub qualifier: String,
    pub organization: String,
    pub application: String,
}

#[derive(Debug)]
pub enum ConfigDir {
    ProjectDir(Label),
    Custom(PathBuf),
}

#[derive(Debug)]
pub struct ConfigSettings {
    pub(crate) config_dir: ConfigDir,
    pub(crate) format: Format,
    pub(crate) config_filename: String,
}

impl ConfigSettings {
    pub fn new(config_dir: ConfigDir, format: Format, config_filename: String) -> Self {
        Self {
            config_dir,
            format,
            config_filename,
        }
    }

    pub fn get_config_dir(&self) -> PathBuf {
        match &self.config_dir {
            ConfigDir::Custom(config_dir) => config_dir.to_owned(),
            ConfigDir::ProjectDir(label) => {
                ProjectDirs::from(&label.qualifier, &label.organization, &label.application)
                    .unwrap()
                    .config_dir()
                    .to_owned()
            }
        }
    }

    pub fn get_full_path(&self) -> PathBuf {
        self.get_config_dir().join(&self.config_filename)
    }
}

pub(crate) fn io_error(msg: &str, inner: io::Error) -> io::Error {
    io::Error::new(inner.kind(), format!("{msg}: {inner}"))
}

use std::fs::create_dir_all;
use std::io::{self};
use std::path::{Path, PathBuf};

pub use ::schematic;
use directories::ProjectDirs;
use schematic::Format;
use serde::{Deserialize, Serialize};
use tracing::debug;
pub use watcher::*;

pub mod backend;

mod watcher;

pub trait LoadConfig {
    type Config: PartialEq;
    type Error;

    fn snapshot(&self) -> Self::Config;

    fn reload(&self) -> Result<Self::Config, Self::Error>;

    fn full_path(&self) -> PathBuf;

    fn directory(&self) -> PathBuf {
        self.full_path().parent().unwrap().to_path_buf()
    }

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

impl ConfigDir {
    pub fn get_config_dir(&self) -> PathBuf {
        match &self {
            Self::Custom(config_dir) => config_dir.to_owned(),
            Self::ProjectDir(label) => {
                ProjectDirs::from(&label.qualifier, &label.organization, &label.application)
                    .unwrap()
                    .config_dir()
                    .to_owned()
            }
        }
    }
}

pub(crate) fn ensure_created<F>(
    config_dir: &Path,
    full_path: &Path,
    template_fn: F,
) -> io::Result<()>
where
    F: FnOnce(),
{
    if full_path.exists() {
        debug!("Not creating config file {full_path:#?} because it already exists");
        return Ok(());
    }
    overwrite_config_file(config_dir, template_fn)
}

pub(crate) fn overwrite_config_file<F>(config_dir: &Path, template_fn: F) -> io::Result<()>
where
    F: FnOnce(),
{
    create_dir_all(config_dir)
        .map_err(|e| io_error(&format!("Error creating config dir {:#?}", config_dir), e))?;

    template_fn();
    Ok(())
}

pub(crate) fn io_error(msg: &str, inner: io::Error) -> io::Error {
    io::Error::new(inner.kind(), format!("{msg}: {inner}"))
}

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use figment::Figment;
use serde::Deserialize;

use crate::{ConfigDir, LoadConfig};

pub trait FigmentConfig {
    fn loader(&self, path: &Path) -> Figment;
}

#[derive(Debug)]
pub struct ConfigSettings<F>
where
    F: FigmentConfig,
{
    config_dir: ConfigDir,
    config_filename: String,
    figment_config: F,
}

impl<F> ConfigSettings<F>
where
    F: FigmentConfig,
{
    pub fn new(config_dir: ConfigDir, config_filename: String, figment_config: F) -> Self {
        Self {
            config_dir,
            config_filename,
            figment_config,
        }
    }

    pub fn get_full_path(&self) -> PathBuf {
        self.config_dir.get_config_dir().join(&self.config_filename)
    }
}

#[derive(Clone)]
pub struct AppConfig<T, F>
where
    F: FigmentConfig,
{
    config_dir: PathBuf,
    filename: String,
    figment_config: F,
    config: Arc<ArcSwap<T>>,
}

impl<'de, T, F> AppConfig<T, F>
where
    T: Deserialize<'de> + PartialEq,
    F: FigmentConfig,
{
    pub fn new(settings: ConfigSettings<F>) -> Self {
        let config_dir = settings.config_dir.get_config_dir();
        let full_path = settings.get_full_path();

        let loader = settings.figment_config.loader(&full_path);
        let val = loader.extract().unwrap();
        let config = Arc::new(ArcSwap::new(Arc::new(val)));

        Self {
            config_dir,
            filename: settings.config_filename,
            figment_config: settings.figment_config,
            config,
        }
    }
}

impl<'de, T, F> LoadConfig for AppConfig<T, F>
where
    T: Deserialize<'de> + PartialEq,
    F: FigmentConfig,
{
    type Config = Arc<T>;
    type Error = Arc<figment::Error>;

    fn snapshot(&self) -> Arc<T> {
        self.config.load_full()
    }

    fn reload(&self) -> Result<Arc<T>, Self::Error> {
        let loader = self.figment_config.loader(&self.full_path());

        let val = loader.extract().map_err(Arc::new)?;
        self.config.store(Arc::new(val));
        Ok(self.snapshot())
    }

    fn full_path(&self) -> PathBuf {
        self.config_dir.join(&self.filename)
    }
}

use std::io::{self};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use schematic::{Config, ConfigError, ConfigLoader, Schematic};

use crate::{ConfigDir, LoadConfig, ensure_created, overwrite_config_file};

pub trait SchematicConfig<T>
where
    T: Config,
{
    fn loader(&self, path: &Path) -> ConfigLoader<T>;
    fn generate_template(&self, path: &Path);
}

pub struct ConfigSettings<T, S>
where
    T: Config,
    S: SchematicConfig<T>,
{
    config_dir: ConfigDir,
    config_filename: String,
    schematic_config: S,
    _phantom: PhantomData<T>,
}

impl<T, S> ConfigSettings<T, S>
where
    T: Config,
    S: SchematicConfig<T>,
{
    pub fn new(config_dir: ConfigDir, config_filename: String, schematic_config: S) -> Self {
        Self {
            config_dir,
            config_filename,
            schematic_config,
            _phantom: PhantomData,
        }
    }

    pub fn get_full_path(&self) -> PathBuf {
        self.config_dir.get_config_dir().join(&self.config_filename)
    }
}

#[derive(Clone)]
pub struct AppConfig<T, S>
where
    S: SchematicConfig<T>,
    T: Config,
{
    config_dir: PathBuf,
    filename: String,
    schematic_config: S,
    config: Arc<ArcSwap<T>>,
}

impl<T, S> LoadConfig for AppConfig<T, S>
where
    T: Config + PartialEq,
    S: SchematicConfig<T>,
{
    type Config = Arc<T>;
    type Error = Arc<ConfigError>;

    fn snapshot(&self) -> Arc<T> {
        self.config.load_full()
    }

    fn reload(&self) -> Result<Arc<T>, Self::Error> {
        let loader = self.schematic_config.loader(&self.full_path());
        let val = loader.load().map_err(Arc::new)?;
        self.config.store(Arc::new(val.config));
        Ok(self.snapshot())
    }

    fn full_path(&self) -> PathBuf {
        self.config_dir.join(&self.filename)
    }
}

impl<T, S> AppConfig<T, S>
where
    S: SchematicConfig<T>,
    T: Schematic + Config + PartialEq,
{
    pub fn new(settings: ConfigSettings<T, S>) -> Result<Self, ConfigError> {
        let config_dir = settings.config_dir.get_config_dir();

        let full_path = settings.get_full_path();
        ensure_created(&config_dir, &full_path, || {
            settings.schematic_config.generate_template(&full_path);
        })
        .unwrap();

        let loader = settings.schematic_config.loader(&full_path);
        let val = loader.load()?.config;
        let config = Arc::new(ArcSwap::new(Arc::new(val)));

        Ok(Self {
            schematic_config: settings.schematic_config,
            config_dir,
            filename: settings.config_filename,
            config,
        })
    }
    pub fn ensure_created(&self) -> io::Result<()> {
        ensure_created(&self.config_dir, &self.full_path(), || {
            self.write_config_template()
        })
    }

    pub fn overwrite_config_file(&self) -> io::Result<()> {
        overwrite_config_file(&self.config_dir, || self.write_config_template())
    }

    pub fn write_config_template(&self) {
        self.schematic_config.generate_template(&self.full_path());
    }
}

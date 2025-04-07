use std::fs::{self, create_dir_all};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use confique::{Config, FileFormat};
use tracing::debug;

use crate::{ConfigDir, LoadConfig, ensure_created, io_error, overwrite_config_file};

#[derive(Debug)]
pub struct ConfigSettings<T>
where
    T: Config,
{
    config_dir: ConfigDir,
    config_filename: String,
    partial: Option<T::Partial>,
}

impl<T> ConfigSettings<T>
where
    T: Config,
{
    pub fn new(config_dir: ConfigDir, config_filename: String) -> Self {
        Self {
            config_dir,
            config_filename,
            partial: None,
        }
    }

    pub fn partial(mut self, partial: T::Partial) -> Self {
        self.partial = Some(partial);
        self
    }

    pub fn get_full_path(&self) -> PathBuf {
        self.config_dir.get_config_dir().join(&self.config_filename)
    }
}

#[derive(Clone)]
pub struct AppConfig<T: Config> {
    config_dir: PathBuf,
    partial: Option<T::Partial>,
    filename: String,
    config: Arc<ArcSwap<T>>,
}

impl<T: Config + PartialEq> LoadConfig for AppConfig<T>
where
    T: Config + PartialEq,
    T::Partial: Clone,
{
    type Config = Arc<T>;
    type Error = Arc<confique::Error>;
    fn snapshot(&self) -> Arc<T> {
        self.config.load_full()
    }

    fn reload(&self) -> Result<Arc<T>, Self::Error> {
        let mut loader = T::builder().env().file(self.full_path());
        if let Some(partial) = &self.partial {
            loader = loader.preloaded(partial.clone());
        }

        let val = loader.load().map_err(Arc::new)?;
        self.config.store(Arc::new(val));
        Ok(self.snapshot())
    }

    fn full_path(&self) -> PathBuf {
        self.config_dir.join(&self.filename)
    }
}

impl<T> AppConfig<T>
where
    T: Config + PartialEq,
    T::Partial: Clone,
{
    pub fn new(settings: ConfigSettings<T>) -> Result<Self, confique::Error> {
        let config_dir = settings.config_dir.get_config_dir();

        let full_path = settings.get_full_path();
        if !full_path.exists() {
            write_config_template::<T>(&full_path);
        }

        let mut loader = T::builder().env().file(full_path);
        if let Some(partial) = &settings.partial {
            loader = loader.preloaded(partial.clone());
        }

        let val = loader.load()?;
        let config = Arc::new(ArcSwap::new(Arc::new(val)));

        Ok(Self {
            config_dir,
            partial: settings.partial,
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
        let path = self.full_path();
        let format = FileFormat::from_extension(path.extension().unwrap()).unwrap();
        let content = match format {
            FileFormat::Toml => {
                confique::toml::template::<T>(confique::toml::FormatOptions::default())
            }
            FileFormat::Yaml => {
                confique::yaml::template::<T>(confique::yaml::FormatOptions::default())
            }
            FileFormat::Json5 => {
                confique::json5::template::<T>(confique::json5::FormatOptions::default())
            }
        };
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }
}

fn write_config_template<T: Config>(path: &Path) {
    let format = FileFormat::from_extension(path.extension().unwrap()).unwrap();
    let content = match format {
        FileFormat::Toml => confique::toml::template::<T>(confique::toml::FormatOptions::default()),
        FileFormat::Yaml => confique::yaml::template::<T>(confique::yaml::FormatOptions::default()),
        FileFormat::Json5 => {
            confique::json5::template::<T>(confique::json5::FormatOptions::default())
        }
    };
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

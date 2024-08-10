use std::fs::create_dir_all;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use schematic::schema::{
    JsoncTemplateRenderer, PklTemplateRenderer, SchemaGenerator, TomlTemplateRenderer,
    YamlTemplateRenderer,
};
use schematic::{Config, ConfigError, ConfigLoader, Format, Schematic};
use tracing::debug;

use crate::{io_error, ConfigSettings, LoadConfig};

#[derive(Clone)]
pub struct AppConfig<T: Config> {
    format: Format,
    config_dir: PathBuf,
    filename: String,
    config: Arc<ArcSwap<T>>,
    // loader: ConfigLoader<T>,
}

impl<T: Config + PartialEq> LoadConfig for AppConfig<T> {
    type Config = Arc<T>;
    type Error = Arc<ConfigError>;
    fn snapshot(&self) -> Arc<T> {
        self.config.load_full()
    }

    fn reload(&self) -> Result<Arc<T>, Self::Error> {
        let mut loader = ConfigLoader::<T>::new();
        loader.file(self.full_path()).unwrap();

        let val = loader.load().map_err(Arc::new)?;
        self.config.store(Arc::new(val.config));
        Ok(self.snapshot())
    }

    fn full_path(&self) -> PathBuf {
        self.config_dir.join(&self.filename)
    }
}

impl<T: Schematic + Config + PartialEq> AppConfig<T> {
    pub fn new(settings: ConfigSettings) -> Self {
        let config_dir = settings.get_config_dir();

        let full_path = settings.get_full_path();
        write_config_template::<T>(settings.format, &full_path);
        let mut loader = ConfigLoader::<T>::new();
        loader.file(full_path).unwrap();
        let val = loader.load().unwrap().config;
        let config = Arc::new(ArcSwap::new(Arc::new(val)));

        Self {
            format: settings.format,
            config_dir,
            filename: settings.config_filename,
            config,
        }
    }

    pub fn ensure_created(&self) -> io::Result<()> {
        let full_path = self.full_path();
        if full_path.exists() {
            debug!("Not creating config file {full_path:#?} because it already exists");
            return Ok(());
        }

        self.overwrite_config_file()
    }

    pub fn overwrite_config_file(&self) -> io::Result<()> {
        create_dir_all(&self.config_dir).map_err(|e| {
            io_error(
                &format!("Error creating config dir {:#?}", self.config_dir),
                e,
            )
        })?;

        self.write_config_template();
        Ok(())
    }

    pub fn write_config_template(&self) {
        write_config_template::<T>(self.format, &self.full_path())
    }
}

fn write_config_template<T: Schematic>(format: Format, path: &Path) {
    let mut generator = SchemaGenerator::default();
    generator.add::<T>();

    match format {
        Format::Json => {
            generator
                .generate(path, JsoncTemplateRenderer::default())
                .unwrap();
        }
        Format::Pkl => {
            generator
                .generate(path, PklTemplateRenderer::default())
                .unwrap();
        }
        Format::Toml => {
            generator
                .generate(path, TomlTemplateRenderer::default())
                .unwrap();
        }
        Format::Yaml => {
            generator
                .generate(path, YamlTemplateRenderer::default())
                .unwrap();
        }
        Format::None => {}
    }
}

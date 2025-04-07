use std::fs::create_dir_all;
use std::io::{self};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use schematic::schema::{
    JsoncTemplateRenderer, PklTemplateRenderer, SchemaGenerator, TomlTemplateRenderer,
    YamlTemplateRenderer,
};
use schematic::{Config, ConfigError, ConfigLoader, Format, PartialConfig, Schematic};
use tracing::debug;

use crate::{ConfigDir, LoadConfig, ensure_created, io_error, overwrite_config_file};

pub struct ConfigSettings<T, C>
where
    T: Config,
{
    config_dir: ConfigDir,
    format: Format,
    config_filename: String,
    partial: Option<T::Partial>,
    context: C,
}

impl<T> ConfigSettings<T, ()>
where
    T: Config,
{
    pub fn new(config_dir: ConfigDir, format: Format, config_filename: String) -> Self {
        Self {
            config_dir,
            format,
            config_filename,
            partial: None,
            context: (),
        }
    }
}

impl<T, C> ConfigSettings<T, C>
where
    T: Config,
{
    pub fn partial(mut self, partial: T::Partial) -> Self {
        self.partial = Some(partial);
        self
    }

    pub fn get_full_path(&self) -> PathBuf {
        self.config_dir.get_config_dir().join(&self.config_filename)
    }
}

impl<T, C> ConfigSettings<T, C>
where
    T: Config + PartialConfig,
{
    pub fn context(
        self,
        context: <T as PartialConfig>::Context,
    ) -> ConfigSettings<T, <T as PartialConfig>::Context> {
        let Self {
            config_dir,
            format,
            config_filename,
            partial,
            context: _context,
        } = self;
        ConfigSettings {
            config_dir,
            format,
            config_filename,
            partial,
            context,
        }
    }
}

#[derive(Clone)]
pub struct AppConfig<T: Config> {
    format: Format,
    config_dir: PathBuf,
    filename: String,
    config: Arc<ArcSwap<T>>,
}

impl<T> LoadConfig for AppConfig<T>
where
    T: Config + PartialEq,
{
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

impl<T> AppConfig<T>
where
    T: Schematic + Config + PartialEq,
{
    pub fn new(
        settings: ConfigSettings<T, <T::Partial as PartialConfig>::Context>,
    ) -> Result<Self, ConfigError> {
        let config_dir = settings.config_dir.get_config_dir();

        let full_path = settings.get_full_path();
        if !full_path.exists() {
            write_config_template::<T>(settings.format, &full_path);
        }

        let mut loader = ConfigLoader::<T>::new();
        loader.file(full_path)?;
        loader.load_partial(&settings.context)?;
        let val = loader.load()?.config;
        let config = Arc::new(ArcSwap::new(Arc::new(val)));

        Ok(Self {
            format: settings.format,
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

use std::fs;
use std::io::{self, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use confique::{Builder, Config};

use crate::{ConfigDir, LoadConfig, ensure_created, overwrite_config_file};

pub trait ConfiqueConfig<T>
where
    T: Config,
{
    fn builder(&self, path: &Path) -> Builder<T>;
    fn template(&self) -> String;
}

#[derive(Debug)]
pub struct ConfigSettings<T, C>
where
    T: Config,
    C: ConfiqueConfig<T>,
{
    config_dir: ConfigDir,
    config_filename: String,
    confique_config: C,
    _phantom: PhantomData<T>,
}

impl<T, C> ConfigSettings<T, C>
where
    T: Config,
    C: ConfiqueConfig<T>,
{
    pub fn new(config_dir: ConfigDir, config_filename: String, confique_config: C) -> Self {
        Self {
            config_dir,
            config_filename,
            confique_config,
            _phantom: PhantomData,
        }
    }

    pub fn get_full_path(&self) -> PathBuf {
        self.config_dir.get_config_dir().join(&self.config_filename)
    }
}

#[derive(Clone)]
pub struct AppConfig<T, C>
where
    T: Config,
    C: ConfiqueConfig<T>,
{
    config_dir: PathBuf,
    confique_config: C,
    filename: String,
    config: Arc<ArcSwap<T>>,
}

impl<T, C> LoadConfig for AppConfig<T, C>
where
    T: Config + PartialEq,
    C: ConfiqueConfig<T>,
{
    type Config = Arc<T>;
    type Error = Arc<confique::Error>;
    fn snapshot(&self) -> Arc<T> {
        self.config.load_full()
    }

    fn reload(&self) -> Result<Arc<T>, Self::Error> {
        let loader = self.confique_config.builder(&self.full_path());
        let val = loader.load().map_err(Arc::new)?;
        self.config.store(Arc::new(val));
        Ok(self.snapshot())
    }

    fn full_path(&self) -> PathBuf {
        self.config_dir.join(&self.filename)
    }
}

impl<T, C> AppConfig<T, C>
where
    T: Config + PartialEq,
    C: ConfiqueConfig<T>,
{
    pub fn new(settings: ConfigSettings<T, C>) -> Result<Self, confique::Error> {
        let config_dir = settings.config_dir.get_config_dir();

        let full_path = settings.get_full_path();
        ensure_created(&config_dir, &full_path, || {
            write_config_template(&settings.confique_config, &full_path);
        })
        .unwrap();

        let loader = settings.confique_config.builder(&settings.get_full_path());
        let val = loader.load()?;
        let config = Arc::new(ArcSwap::new(Arc::new(val)));

        Ok(Self {
            config_dir,
            confique_config: settings.confique_config,
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
        write_config_template(&self.confique_config, &self.full_path());
    }
}

fn write_config_template<T, C>(confique_config: &C, path: &Path)
where
    T: Config,
    C: ConfiqueConfig<T>,
{
    let content = confique_config.template();
    let mut file = fs::File::create(path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
}

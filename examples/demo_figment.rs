use std::path::Path;

use clap::Parser;
use figment::Figment;
use figment::providers::{Format, Yaml};
use serde::Deserialize;
use tracing_subscriber::EnvFilter;
use watch_config::backend::figment::{AppConfig, ConfigSettings, FigmentConfig};
use watch_config::{ConfigDir, ConfigWatcherService, LoadConfig};

#[derive(Deserialize, PartialEq, Eq, Clone, Debug)]
struct AppConfigExample {
    pub number: usize,
    pub string: String,
    pub boolean: bool,
    pub array: Vec<String>,
    pub optional: Option<String>,
}

struct ConfigBuilder;

impl FigmentConfig for ConfigBuilder {
    fn loader(&self, path: &Path) -> figment::Figment {
        Figment::new().merge(Yaml::file(path))
    }
}

#[derive(Parser, Clone)]
enum Cli {
    Watch,
    Path,
    Edit,
    Validate,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_line_number(true)
        .with_file(true)
        .init();

    let settings = ConfigSettings::new(
        ConfigDir::Custom("./.config".into()),
        "config_figment.yml".to_owned(),
        ConfigBuilder,
    );

    let config = AppConfig::<AppConfigExample, _>::new(settings);

    let cli = Cli::parse();
    match cli {
        Cli::Watch => {
            let watcher = ConfigWatcherService::new(config);
            let handle = watcher.handle();
            watcher.spawn();
            let mut events = handle.subscribe();
            while let Ok(event) = events.recv().await {
                match event {
                    Ok(event) => {
                        println!("Old {:?}", event.old);
                        println!("New {:?}", event.new);
                    }
                    Err(e) => {
                        println!("{e:?}");
                    }
                }
            }
        }
        Cli::Edit => {
            config.edit().unwrap();
        }
        Cli::Path => {
            let path = config.full_path().to_string_lossy().to_string();
            println!("{path}");
        }
        Cli::Validate => {
            if let Err(e) = config.reload() {
                println!("{e:?}");
            } else {
                println!("valid");
            }
        }
    }
}

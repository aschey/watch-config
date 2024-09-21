use clap::Parser;
use schematic::{Config, Format};
use tracing_subscriber::EnvFilter;
use watch_config::backend::schematic::AppConfig;
use watch_config::{ConfigDir, ConfigSettings, ConfigWatcherService, LoadConfig};

#[derive(Config, PartialEq, Eq, Clone, Debug)]
struct AppConfigExample {
    pub number: usize,
    pub string: String,
    pub boolean: bool,
    pub array: Vec<String>,
    pub optional: Option<String>,
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

    let config = AppConfig::<AppConfigExample>::new(ConfigSettings::new(
        ConfigDir::Custom("./.config".into()),
        Format::Yaml,
        "config.yml".to_owned(),
    ));

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

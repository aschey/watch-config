use clap::Parser;
use confique::Config;
use tracing_subscriber::EnvFilter;
use watch_config::backend::confique::{AppConfig, ConfigSettings};
use watch_config::{ConfigDir, ConfigWatcherService, LoadConfig};

#[derive(Config, PartialEq, Eq, Clone, Debug)]
#[config(partial_attr(derive(Clone)))]
struct AppConfigExample {
    #[config(default = 1)]
    pub number: usize,
    #[config(default = "abc")]
    pub string: String,
    #[config(default = true)]
    pub boolean: bool,
    #[config(default=[])]
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

    let settings = ConfigSettings::new(
        ConfigDir::Custom("./.config".into()),
        "config.yml".to_owned(),
    );
    let config = AppConfig::<AppConfigExample>::new(settings).unwrap();

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

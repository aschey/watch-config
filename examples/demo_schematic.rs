use clap::Parser;
use schematic::schema::{SchemaGenerator, YamlTemplateRenderer};
use schematic::{Config, ConfigLoader};
use tracing_subscriber::EnvFilter;
use watch_config::backend::schematic::{AppConfig, ConfigSettings, SchematicConfig};
use watch_config::{ConfigDir, ConfigWatcherService, LoadConfig};

#[derive(Config, PartialEq, Eq, Clone, Debug)]
struct AppConfigExample {
    #[setting(default = 1)]
    pub number: usize,
    #[setting(default = "abc")]
    pub string: String,
    #[setting(default = true)]
    pub boolean: bool,
    #[setting(default=vec![])]
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

struct ConfigBuilder;

impl SchematicConfig<AppConfigExample> for ConfigBuilder {
    fn loader(&self, path: &std::path::Path) -> schematic::ConfigLoader<AppConfigExample> {
        let mut loader = ConfigLoader::<AppConfigExample>::new();
        loader.file(path).unwrap();
        loader
    }

    fn generate_template(&self, path: &std::path::Path) {
        let mut generator = SchemaGenerator::default();
        generator.add::<AppConfigExample>();
        generator
            .generate(path, YamlTemplateRenderer::default())
            .unwrap();
    }
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
        "config_schematic.yml".to_owned(),
        ConfigBuilder,
    );
    let config = AppConfig::new(settings).unwrap();

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

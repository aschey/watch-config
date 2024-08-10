use futures_cancel::FutureExt;
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer_opt, DebouncedEvent};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use crate::LoadConfig;

#[derive(Clone, Debug)]
pub struct ConfigUpdate<T> {
    pub old: T,
    pub new: T,
}

pub struct ConfigServiceHandle<T: LoadConfig> {
    tx: broadcast::Sender<Result<ConfigUpdate<T::Config>, T::Error>>,
    cancellation_token: CancellationToken,
}

impl<T: LoadConfig> ConfigServiceHandle<T> {
    pub fn subscribe(&self) -> broadcast::Receiver<Result<ConfigUpdate<T::Config>, T::Error>> {
        self.tx.subscribe()
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }
}

#[derive(Debug)]
pub struct ConfigWatcherService<T>
where
    T: LoadConfig,
{
    config: T,
    cancellation_token: CancellationToken,
    config_tx: broadcast::Sender<Result<ConfigUpdate<T::Config>, T::Error>>,
}

impl<T> ConfigWatcherService<T>
where
    T: LoadConfig + Send + Sync + 'static,
    T::Config: Clone + Send,
    T::Error: Clone + Send,
{
    pub fn new(config: T) -> Self {
        let (config_tx, _) = broadcast::channel(32);
        Self {
            config,
            config_tx,
            cancellation_token: CancellationToken::new(),
        }
    }

    pub async fn run(self) {
        let (file_changed_tx, mut file_changed_rx) = watch::channel(());
        let mut debouncer = new_debouncer_opt::<_, RecommendedWatcher>(
            notify_debouncer_mini::Config::default(),
            move |events: Result<Vec<DebouncedEvent>, notify::Error>| {
                if events
                    .inspect_err(|e| error!("File watch error: {e:?}"))
                    .is_ok()
                {
                    file_changed_tx
                        .send(())
                        .inspect_err(|e| warn!("Error sending file paths: {e:?}"))
                        .ok();
                }
            },
        )
        .unwrap();
        let watcher = debouncer.watcher();
        watcher
            .watch(&self.config.full_path(), RecursiveMode::NonRecursive)
            .unwrap();

        while let Ok(Ok(_)) = file_changed_rx
            .changed()
            .cancel_on_shutdown(&self.cancellation_token)
            .await
        {
            let old = self.config.snapshot();
            match self.config.reload() {
                Ok(new) => {
                    if old != new {
                        self.config_tx.send(Ok(ConfigUpdate { old, new })).ok();
                    }
                }
                Err(e) => {
                    self.config_tx.send(Err(e)).ok();
                }
            }
        }
    }

    pub fn handle(&self) -> ConfigServiceHandle<T> {
        ConfigServiceHandle {
            tx: self.config_tx.clone(),
            cancellation_token: self.cancellation_token.clone(),
        }
    }

    pub fn spawn(self) -> JoinHandle<()> {
        tokio::spawn(async move { self.run().await })
    }
}

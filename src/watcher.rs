use std::time::Duration;

use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;
use tokio_util::future::FutureExt;
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
        let cancellation_token = CancellationToken::new();
        Self {
            config,
            config_tx,
            cancellation_token,
        }
    }

    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    pub async fn run(self) {
        let (file_changed_tx, mut file_changed_rx) = watch::channel(());
        let mut debouncer = new_debouncer(
            Duration::from_secs(1),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result.inspect_err(|e| error!("File watch error: {e:?}"))
                    && events
                        .into_iter()
                        .any(|e| !matches!(e.event.kind, EventKind::Access(_)))
                {
                    file_changed_tx
                        .send(())
                        .inspect_err(|e| warn!("Error sending file paths: {e:?}"))
                        .ok();
                }
            },
        )
        .unwrap();
        debouncer
            .watch(self.config.directory(), RecursiveMode::Recursive)
            .unwrap();

        let mut is_errored = false;
        while let Some(Ok(_)) = file_changed_rx
            .changed()
            .with_cancellation_token(&self.cancellation_token)
            .await
        {
            let old = self.config.snapshot();
            match self.config.reload() {
                Ok(new) => {
                    if is_errored || old != new {
                        self.config_tx.send(Ok(ConfigUpdate { old, new })).ok();
                        is_errored = false;
                    }
                }
                Err(e) => {
                    is_errored = true;
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

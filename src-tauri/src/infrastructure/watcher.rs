use crate::application::service::AppService;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::{path::Path, sync::{mpsc, Arc}, thread, time::Duration};
use tauri::{AppHandle, Emitter};

pub struct WatcherManager {
    watcher: Option<RecommendedWatcher>,
}

impl WatcherManager {
    pub fn new() -> Self { Self { watcher: None } }

    pub fn watch(&mut self, path: &Path, service: Arc<AppService>, app: AppHandle) -> notify::Result<()> {
        self.watcher = None;
        let (tx, rx) = mpsc::channel::<()>();
        let tx_events = tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                if let Ok(event) = result {
                    let relevant = event.paths.iter().any(|p| {
                        let s = p.file_name().and_then(|x| x.to_str()).unwrap_or("").to_ascii_lowercase();
                        s.ends_with(".jar") || s.ends_with(".jar.disabled")
                    });
                    if relevant { let _ = tx_events.send(()); }
                }
            },
            Config::default(),
        )?;
        watcher.watch(path, RecursiveMode::NonRecursive)?;

        thread::spawn(move || {
            while rx.recv().is_ok() {
                while rx.recv_timeout(Duration::from_millis(220)).is_ok() {}
                if let Ok(snapshot) = service.rescan_snapshot() {
                    let _ = app.emit("mods://changed", snapshot);
                }
            }
        });

        self.watcher = Some(watcher);
        Ok(())
    }
}

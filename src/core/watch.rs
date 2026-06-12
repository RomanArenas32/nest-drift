use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

use notify::{recommended_watcher, Event, RecursiveMode, Watcher};

/// Handle returned by `watch`. Call `.stop()` to shut down the watcher thread.
pub struct WatchHandle {
    stop: Arc<AtomicBool>,
}

impl WatchHandle {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Consume the handle, returning the underlying stop flag.
    /// Useful for storing the flag externally (e.g. in a registry).
    pub fn into_stop_flag(self) -> Arc<AtomicBool> {
        self.stop
    }
}

/// Watches `path` recursively for `.ts` file changes (excluding node_modules/dist).
/// Calls `on_change` after `debounce_ms` of inactivity following a burst of changes.
/// Returns a handle that can stop the watcher.
pub fn watch<F>(path: &str, debounce_ms: u64, on_change: F) -> WatchHandle
where
    F: Fn() + Send + 'static,
{
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = stop.clone();
    let path = path.to_string();

    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<()>();

        let mut watcher = recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let is_relevant = event.paths.iter().any(|p| {
                    let s = p.to_string_lossy();
                    p.extension().map(|e| e == "ts").unwrap_or(false)
                        && !s.contains("node_modules")
                        && !s.contains("/dist/")
                        && !s.contains("\\dist\\")
                });
                if is_relevant {
                    let _ = tx.send(());
                }
            }
        })
        .expect("Failed to create file watcher");

        watcher
            .watch(std::path::Path::new(&path), RecursiveMode::Recursive)
            .expect("Failed to watch path");

        let mut pending: Option<Instant> = None;

        loop {
            if stop_clone.load(Ordering::Relaxed) {
                break;
            }

            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(_) => {
                    // drain the burst, reset the debounce timer
                    while rx.try_recv().is_ok() {}
                    pending = Some(Instant::now());
                }
                Err(_) => {
                    if let Some(t) = pending {
                        if t.elapsed().as_millis() >= debounce_ms as u128 {
                            pending = None;
                            on_change();
                        }
                    }
                }
            }
        }
    });

    WatchHandle { stop }
}

use crate::{
    config::{Source, Sources, Config},
    db::Database,
    dl::{self, Media, MediaEntry},
    error::Error,
    gui,
};
use crossbeam_channel::{unbounded, Receiver};
use tracing::{error, info};
use notify::{Event, INotifyWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::{
    collections::VecDeque,
    fs,
    path::PathBuf,
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use tokio::task::JoinHandle as Task;

const UPDATE_INTERVAL: u64 = 500;
const DIR_NAME: &str = "drainpipe";

pub struct State {
    pub config: Config,
    pub sources: Sources,
    pub dl_queue: VecDeque<MediaEntry>,
}

pub struct Daemon {
    config_path: PathBuf,
    sources_path: PathBuf,
    fs_event_rx: Receiver<Result<Event, notify::Error>>,
    _watcher: INotifyWatcher,
    dl_tasks: Vec<JoinHandle<Result<Media, String>>>,
    sync_task: Option<Task<Vec<MediaEntry>>>,
    last_sync: Option<Instant>,
    state: Arc<Mutex<State>>,
    db: Database,
}

impl Daemon {
    pub async fn start() -> Result<Self, Error> {
        let config_dir = dirs::config_dir().unwrap().join(DIR_NAME);
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.yaml");
        let sources_path = config_dir.join("sources.yaml");
        let config = Config::load(&config_path)?;
        let sources = Sources::load(&sources_path)?;
        let state = State {
            config,
            sources,
            dl_queue: VecDeque::new(),
        };

        let (event_tx, event_rx) = unbounded();
        let mut watcher = RecommendedWatcher::new(event_tx, notify::Config::default())?;
        if !state.config.from_env {
            watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
        }
        watcher.watch(&sources_path, RecursiveMode::NonRecursive)?;

        fs::create_dir_all(&state.config.data.media_dir)?;

        let data_dir = dirs::data_dir().unwrap().join(DIR_NAME);
        fs::create_dir_all(&data_dir)?;
        let db = Database::load(&data_dir.join("library.db")).await?;

        let port = state.config.data.port;
        let state = Arc::new(Mutex::new(state));
        gui::start(port, Arc::new(db.clone()), state.clone());

        Ok(Self {
            config_path,
            sources_path,
            fs_event_rx: event_rx,
            _watcher: watcher,
            dl_tasks: Vec::new(),
            last_sync: None,
            sync_task: None,
            state,
            db,
        })
    }

    pub async fn run(mut self) -> Result<(), Error> {
        info!("Daemon started.");
        loop {
            let state = self.state.clone();
            let mut state = state.lock();

            // Receive finished threads
            let mut finished = Vec::with_capacity(self.dl_tasks.len());
            for (n, thread) in self.dl_tasks.iter().enumerate() {
                if thread.is_finished() {
                    finished.push(n);
                }
            }
            for n in finished.into_iter() {
                let thread = self.dl_tasks.remove(n);
                match thread.join().unwrap() {
                    Ok(media) => {
                        info!("Downloaded '{}' to '{}'", media.title, media.path);
                        self.db.insert(&media).await?;
                    }
                    Err(e) => error!("Download failed: {e}"),
                }
            }

            // Receive file updates
            if let Ok(event) = self.fs_event_rx.try_recv() {
                let event = event?;
                if !event.kind.is_access() {
                    if event.paths.contains(&self.config_path) {
                        state.config.reload()?;
                    } else if event.paths.contains(&self.sources_path) {
                        state.sources.reload()?;
                        // Sync on sources update
                        if self.last_sync.is_none() {
                            self.start_sync(state.sources.get());
                        }
                    }
                }
            }

            // Receive sync
            if let Some(task) = &self.sync_task {
                if task.is_finished() {
                    let task = self.sync_task.take().unwrap();
                    match task.await {
                        Ok(mut entries) => {
                            info!("Got {} entries from sync", entries.len());
                            // Apply download filter
                            if let Some(filter) = &state.config.data.download_filter {
                                entries.retain(|e| !filter.filter(e));
                            }
                            // Check if not alreaday downloaded
                            for e in entries {
                                // TODO: Prevent blocking here
                                if self.db.get(&e.link).await?.is_none() {
                                    info!("Add '{}' to queue", e.link);
                                    state.dl_queue.push_back(e);
                                }
                            }
                        }
                        Err(e) => error!("Failed sync: {e}"),
                    }
                }
            } else if self
                .last_sync
                .map(|v| v.elapsed().as_secs() > state.config.data.sync_interval)
                .unwrap_or(true)
            {
                self.start_sync(state.sources.get());
            }

            // Start downloads
            for _ in 0..state
                .config
                .data
                .parallel_downloads
                .saturating_sub(self.dl_tasks.len() as u64)
            {
                if let Some(item) = state.dl_queue.pop_front() {
                    info!(
                        "Downloading {}  {}",
                        item.title.unwrap_or_else(|| "Unkown Title".to_string()),
                        item.link
                    );
                    self.dl_tasks.push(dl::download_video(
                        state.config.data.media_dir.to_string_lossy().to_string(),
                        item.link,
                    ));
                }
            }
            thread::sleep(Duration::from_millis(UPDATE_INTERVAL));
        }
    }

    fn start_sync(&mut self, sources: Vec<Source>) {
        info!("Sarting sync..");
        self.sync_task = Some(tokio::spawn(dl::crawl_sources(sources)));
        self.last_sync = Some(Instant::now());
    }
}

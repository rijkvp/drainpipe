use crate::{
    config::{Config, Loadable, Sources},
    db::Database,
    dl::{self, Media, MediaEntry},
    error::Error,
    gui,
};
use crossbeam_channel::{unbounded, Receiver};
use log::{error, info};
use notify::{Event, INotifyWatcher, RecommendedWatcher, RecursiveMode, Watcher};
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

pub struct Daemon {
    config: Config,
    config_path: PathBuf,
    sources: Sources,
    sources_path: PathBuf,
    fs_event_rx: Receiver<Result<Event, notify::Error>>,
    _watcher: INotifyWatcher,
    dl_tasks: Vec<JoinHandle<Result<Media, String>>>,
    dl_queue: VecDeque<MediaEntry>,
    sync_task: Option<Task<Vec<MediaEntry>>>,
    last_sync: Option<Instant>,
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

        let (event_tx, event_rx) = unbounded();
        let mut watcher = RecommendedWatcher::new(event_tx, notify::Config::default())?;
        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;
        watcher.watch(&sources_path, RecursiveMode::NonRecursive)?;

        fs::create_dir_all(&config.media_dir)?;

        let data_dir = dirs::data_dir().unwrap().join(DIR_NAME);
        fs::create_dir_all(&data_dir)?;
        let db = Database::load(&data_dir.join("library.db")).await?;
        gui::start(config.port, Arc::new(db.clone()));

        Ok(Self {
            config,
            config_path,
            sources,
            sources_path,
            fs_event_rx: event_rx,
            _watcher: watcher,
            dl_tasks: Vec::new(),
            dl_queue: VecDeque::new(),
            last_sync: None,
            sync_task: None,
            db,
        })
    }

    pub async fn run(mut self) -> Result<(), Error> {
        info!("Daemon started.");
        loop {
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
                        // TODO: Block
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
                        match Config::reload(&self.config_path) {
                            Ok(new) => self.config = new,
                            Err(e) => error!("Failed to reload config: {e}"),
                        }
                    } else if event.paths.contains(&self.sources_path) {
                        match Sources::reload(&self.sources_path) {
                            Ok(new) => self.sources = new,
                            Err(e) => error!("Failed to reload sources: {e}"),
                        }
                        // Sync on sources update
                        if self.last_sync.is_none() {
                            self.start_sync();
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
                            // Apply download filter
                            if let Some(filter) = &self.config.download_filter {
                                entries.retain(|e| !filter.filter(e));
                            }
                            // Check if not alreaday downloaded
                            for e in entries {
                                // TODO: Prevent blocking here
                                if self.db.get(&e.link).await?.is_none() {
                                    info!("Add '{}' to queue", e.link);
                                    self.dl_queue.push_back(e);
                                }
                            }
                        }
                        Err(e) => error!("Failed sync: {e}"),
                    }
                }
            } else if self
                .last_sync
                .map(|v| v.elapsed().as_secs() > self.config.sync_interval)
                .unwrap_or(true)
            {
                self.start_sync();
            }

            // Start downloads
            for _ in 0..self
                .config
                .parallel_downloads
                .saturating_sub(self.dl_tasks.len() as u64)
            {
                if let Some(item) = self.dl_queue.pop_front() {
                    info!(
                        "Downloading {}  {}",
                        item.title.unwrap_or_else(|| "Unkown Title".to_string()),
                        item.link
                    );
                    self.dl_tasks.push(dl::download_video(
                        self.config.media_dir.to_string_lossy().to_string(),
                        item.link,
                    ));
                }
            }

            thread::sleep(Duration::from_millis(UPDATE_INTERVAL));
        }
    }

    fn start_sync(&mut self) {
        info!("Sarting sync..");
        let sources = self.sources.clone();
        self.sync_task = Some(tokio::spawn(dl::crawl_sources(sources)));
        self.last_sync = Some(Instant::now());
    }
}

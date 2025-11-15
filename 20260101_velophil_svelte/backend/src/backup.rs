use anyhow::Result;
use chrono::Utc;
use log::{error, info, warn};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;

pub struct BackupManager {
    db_path: PathBuf,
    backup_dir: PathBuf,
    name_template: String,
    lock: Arc<Mutex<()>>,
}

impl BackupManager {
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(
        db_path: P,
        backup_dir: Q,
        name_template: &str,
    ) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
            backup_dir: backup_dir.as_ref().to_path_buf(),
            name_template: name_template.to_string(),
            lock: Arc::new(Mutex::new(())),
        }
    }

    #[allow(dead_code)]
    pub fn lock_handle(&self) -> Arc<Mutex<()>> {
        self.lock.clone()
    }

    pub async fn run(self: Arc<Self>, interval: Duration, retention: usize) {
        tokio::fs::create_dir_all(&self.backup_dir).await.ok();
        loop {
            tokio::time::sleep(interval).await;

            let lock = self.lock.clone();
            // 5 minutes timeout to acquire lock
            let acquired = tokio::time::timeout(Duration::from_secs(300), lock.lock()).await;
            if acquired.is_ok() {
                let mut attempts = 0;
                let mut last_err: Option<anyhow::Error> = None;
                while attempts < 3 {
                    attempts += 1;
                    match tokio::time::timeout(Duration::from_secs(180), self.do_backup(retention))
                        .await
                    {
                        Ok(Ok(())) => {
                            info!("Sled backup completed (attempt {} of 3)", attempts);
                            last_err = None;
                            break;
                        }
                        Ok(Err(e)) => {
                            warn!("Sled backup error on attempt {}: {}", attempts, e);
                            last_err = Some(e);
                        }
                        Err(_) => {
                            warn!("Sled backup timed out on attempt {}", attempts);
                            last_err = Some(anyhow::anyhow!("timeout"));
                        }
                    }
                }
                if let Some(e) = last_err {
                    error!("Sled backup failed after 3 attempts: {}", e);
                }
            } else {
                warn!("Backup skipped: another backup in progress for >5 minutes");
            }
        }
    }

    async fn do_backup(&self, retention: usize) -> Result<()> {
        let ts = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let db_path = self.db_path.clone();
        let name = self.name_template.replace("{{timestamp}}", &ts);
        let dst = self.backup_dir.join(name);
        // perform blocking copy in a blocking thread
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&dst)?;
            copy_dir_recursive_sync(&db_path, &dst)
        })
        .await??;
        self.prune_old_backups(retention).await?;
        Ok(())
    }

    fn name_prefix(&self) -> String {
        match self.name_template.split("{{timestamp}}").next() {
            Some(p) => p.to_string(),
            None => String::new(),
        }
    }

    async fn prune_old_backups(&self, keep: usize) -> Result<()> {
        let prefix = self.name_prefix();
        let mut entries = tokio::fs::read_dir(&self.backup_dir).await?;
        let mut items: Vec<(String, PathBuf)> = Vec::new();
        while let Some(e) = entries.next_entry().await? {
            let name = e.file_name().to_string_lossy().to_string();
            // only consider backup dirs we created
            if !prefix.is_empty() && !name.starts_with(&prefix) {
                continue;
            }
            if let Ok(md) = e.metadata().await {
                if md.is_dir() {
                    items.push((name, e.path()));
                }
            }
        }
        // Sort by name (timestamp included) ascending, so oldest first
        items.sort_by(|a, b| a.0.cmp(&b.0));
        let remove_count = items.len().saturating_sub(keep);
        for (_name, path) in items.iter().take(remove_count) {
            let _ = tokio::fs::remove_dir_all(path).await;
        }
        Ok(())
    }

    /// Get the latest backup directory (sorted by timestamp in filename)
    pub async fn get_latest_backup(&self) -> Result<Option<PathBuf>> {
        let prefix = self.name_prefix();

        let mut entries = match tokio::fs::read_dir(&self.backup_dir).await {
            Ok(e) => e,
            Err(err) => {
                warn!("Failed to read backup directory: {}", err);
                return Ok(None);
            }
        };

        let mut items: Vec<(String, PathBuf)> = Vec::new();

        loop {
            let entry = match entries.next_entry().await {
                Ok(Some(e)) => e,
                Ok(None) => break,
                Err(err) => {
                    warn!("Error reading backup directory entry: {}", err);
                    continue;
                }
            };

            let name = entry.file_name().to_string_lossy().to_string();

            // only consider backup dirs we created
            if !prefix.is_empty() && !name.starts_with(&prefix) {
                continue;
            }

            let metadata = match entry.metadata().await {
                Ok(md) => md,
                Err(err) => {
                    warn!("Failed to read metadata for {}: {}", name, err);
                    continue;
                }
            };

            if metadata.is_dir() {
                items.push((name, entry.path()));
            }
        }

        if items.is_empty() {
            info!("No backup directories found");
            return Ok(None);
        }

        // Sort by name (timestamp included) descending, so latest first
        items.sort_by(|a, b| b.0.cmp(&a.0));
        info!("Found {} backups, latest: {}", items.len(), items[0].0);
        Ok(Some(items[0].1.clone()))
    }

    /// Restore database from the latest backup
    pub async fn restore_from_latest(&self) -> Result<bool> {
        let backup_path = match self.get_latest_backup().await {
            Ok(Some(path)) => {
                info!("Latest backup found: {:?}", path);
                path
            }
            Ok(None) => {
                info!("No backups found to restore from");
                return Ok(false);
            }
            Err(err) => {
                error!("Failed to get latest backup: {}", err);
                return Ok(false);
            }
        };

        info!("Attempting to restore database from backup: {:?}", backup_path);

        // Check if database already exists and has data
        let db_exists = match tokio::fs::metadata(&self.db_path).await {
            Ok(_) => true,
            Err(_) => {
                info!("Database does not exist, will restore from backup");
                false
            }
        };

        if db_exists {
            // Database exists, check if it's empty or corrupted
            let db_path = self.db_path.clone();
            let check_result = tokio::task::spawn_blocking(move || {
                match std::fs::read_dir(&db_path) {
                    Ok(mut entries) => {
                        let has_files = entries.next().is_some();
                        info!("Database directory has files: {}", has_files);
                        has_files
                    }
                    Err(err) => {
                        warn!("Failed to read database directory: {}", err);
                        false
                    }
                }
            })
            .await;

            let db_has_data = match check_result {
                Ok(result) => result,
                Err(err) => {
                    error!("Failed to check database contents: {}", err);
                    return Ok(false);
                }
            };

            if db_has_data {
                info!("Database already has data, skipping restore");
                return Ok(false);
            } else {
                info!("Database directory is empty, proceeding with restore");
            }
        }

        // Create database directory if it doesn't exist
        match tokio::fs::create_dir_all(&self.db_path).await {
            Ok(_) => {
                info!("Database directory created/verified");
            }
            Err(err) => {
                error!("Failed to create database directory: {}", err);
                return Err(err.into());
            }
        };

        // Restore backup by copying to database location
        let src = backup_path.clone();
        let dst = self.db_path.clone();

        let copy_result = tokio::task::spawn_blocking(move || {
            info!("Starting copy from {:?} to {:?}", src, dst);
            copy_dir_recursive_sync(&src, &dst)
        })
        .await;

        match copy_result {
            Ok(Ok(())) => {
                info!("Database restored successfully from backup: {:?}", backup_path);
                Ok(true)
            }
            Ok(Err(err)) => {
                error!("Failed to copy backup files: {}", err);
                Err(anyhow::anyhow!("Backup copy failed: {}", err))
            }
            Err(err) => {
                error!("Task execution failed: {}", err);
                Err(anyhow::anyhow!("Backup task failed: {}", err))
            }
        }
    }
}

fn copy_dir_recursive_sync(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    for entry_res in std::fs::read_dir(src)? {
        let entry = entry_res?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            std::fs::create_dir_all(&dst_path)?;
            copy_dir_recursive_sync(&src_path, &dst_path)?;
        } else if ty.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

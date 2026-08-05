use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rand::Rng;
use stealth_oxide::ProfileSeed;

pub struct GeneratedProfile {
    path: PathBuf,
    keep: bool,
    removed: bool,
}

impl GeneratedProfile {
    pub fn create(keep: bool) -> Result<Self> {
        let suffix: u64 = rand::rng().random();
        let path = std::env::temp_dir().join(format!(
            "stealth-oxide-profile-{}-{suffix:016x}",
            std::process::id()
        ));
        std::fs::create_dir(&path)
            .with_context(|| format!("failed to create generated profile at {}", path.display()))?;
        Ok(Self {
            path,
            keep,
            removed: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        if self.keep || self.removed {
            return Ok(());
        }
        let mut last_error = None;
        for _ in 0..40 {
            match std::fs::remove_dir_all(&self.path) {
                Ok(()) => {
                    self.removed = true;
                    return Ok(());
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    self.removed = true;
                    return Ok(());
                }
                Err(error) => last_error = Some(error),
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        Err(last_error
            .map(anyhow::Error::from)
            .unwrap_or_else(|| anyhow::anyhow!("generated profile cleanup failed")))
        .with_context(|| format!("failed to remove generated profile {}", self.path.display()))
    }
}

impl Drop for GeneratedProfile {
    fn drop(&mut self) {
        if !self.keep
            && !self.removed
            && let Err(error) = std::fs::remove_dir_all(&self.path)
        {
            eprintln!(
                "failed to remove generated profile {}: {error}",
                self.path.display()
            );
        }
    }
}

pub fn load_seed_documents(paths: &[PathBuf], target_url: &str) -> Result<Vec<ProfileSeed>> {
    if paths.is_empty() {
        return ProfileSeed::defaults_for(target_url).map_err(anyhow::Error::from);
    }
    paths
        .iter()
        .map(|path| {
            let contents = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read seed file {}", path.display()))?;
            ProfileSeed::from_json(&contents)
                .with_context(|| format!("invalid seed file {}", path.display()))
        })
        .collect()
}

//! Crash-recovery state.
//!
//! Whenever the helper takes an action that needs reversing, it records the
//! action in this file *before* doing the work. On startup, we read the file
//! and reverse anything that's still flagged as in-place.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::Mutex;

#[cfg(unix)]
const STATE_PATH: &str = "/var/db/com.sofriendly.crosspoint.unlocker.helper.state.json";

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct HelperState {
    pub internet_sharing_active: bool,
    pub pfctl_anchor_loaded: bool,
    /// Windows-only: tracks whether the helper has appended spoofing entries to
    /// the system hosts file. Needed because Windows' ICS DNS proxy owns port
    /// 53 on the bridge IP, so we redirect lookups via the hosts file rather
    /// than binding our own DNS.
    #[serde(default)]
    pub hosts_modified: bool,
}

static LOCK: Mutex<()> = Mutex::const_new(());

pub fn path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from(STATE_PATH)
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        base.join("CrossPoint")
            .join("unlocker-helper")
            .join("state.json")
    }
}

pub async fn read() -> anyhow::Result<HelperState> {
    let _g = LOCK.lock().await;
    match fs::read(path()).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(HelperState::default()),
        Err(e) => Err(e.into()),
    }
}

pub async fn write(s: &HelperState) -> anyhow::Result<()> {
    let _g = LOCK.lock().await;
    if let Some(parent) = path().parent() {
        fs::create_dir_all(parent).await.ok();
    }
    let bytes = serde_json::to_vec_pretty(s)?;
    fs::write(path(), bytes).await?;
    Ok(())
}

pub async fn mutate<F: FnOnce(&mut HelperState)>(f: F) -> anyhow::Result<()> {
    let mut s = read().await?;
    f(&mut s);
    write(&s).await
}

/// On helper start: reverse anything left in place by a prior crash.
pub async fn recover() -> anyhow::Result<()> {
    let s = read().await?;
    if s.pfctl_anchor_loaded {
        tracing::warn!("recovering: removing leftover pfctl anchor");
        let _ = crate::ops::pfctl_remove().await;
    }
    if s.internet_sharing_active {
        tracing::warn!("recovering: stopping leftover Internet Sharing");
        let _ = crate::ops::is_disable().await;
    }
    #[cfg(windows)]
    if s.hosts_modified {
        tracing::warn!("recovering: removing leftover hosts file entries");
        let _ = crate::ops::hosts_disarm().await;
    }
    Ok(())
}

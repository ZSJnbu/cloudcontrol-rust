use crate::device::adb::Adb;
use dashmap::DashMap;
use std::sync::Arc;

const SCRCPY_SERVER_LOCAL: &str = "resources/scrcpy/scrcpy-server.jar";
const SCRCPY_SERVER_REMOTE: &str = "/data/local/tmp/scrcpy-server.jar";

/// Tracks which devices already have scrcpy-server.jar pushed.
#[derive(Clone)]
pub struct ScrcpyManager {
    /// serial → true if JAR has been pushed in this session
    pushed: Arc<DashMap<String, bool>>,
}

impl ScrcpyManager {
    pub fn new() -> Self {
        Self {
            pushed: Arc::new(DashMap::new()),
        }
    }

    /// Ensure scrcpy-server.jar is pushed to the device. Skips if already done this session.
    pub async fn ensure_scrcpy_ready(&self, serial: &str) -> Result<(), String> {
        if self.pushed.contains_key(serial) {
            return Ok(());
        }

        // Check if JAR exists locally
        if !std::path::Path::new(SCRCPY_SERVER_LOCAL).exists() {
            return Err("scrcpy-server.jar not found locally".to_string());
        }

        tracing::info!("[ScrcpyManager] Pushing scrcpy-server.jar to {}", serial);
        Adb::push(serial, SCRCPY_SERVER_LOCAL, SCRCPY_SERVER_REMOTE).await?;
        self.pushed.insert(serial.to_string(), true);
        tracing::info!(
            "[ScrcpyManager] scrcpy-server.jar pushed to {}",
            serial
        );

        Ok(())
    }

    /// Remove tracking for a device (e.g. when it disconnects).
    pub fn remove_device(&self, serial: &str) {
        self.pushed.remove(serial);
    }

    /// Check if scrcpy-server.jar is available locally.
    pub fn jar_available() -> bool {
        std::path::Path::new(SCRCPY_SERVER_LOCAL).exists()
    }
}

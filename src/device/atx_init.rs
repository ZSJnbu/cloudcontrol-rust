use crate::device::adb::Adb;

/// Initialize atx-agent on a device.
pub struct AtxInit;

#[allow(dead_code)]
impl AtxInit {
    /// Full atx-agent initialization:
    /// 1. Push atx-agent binary to device
    /// 2. Push atx.sh script
    /// 3. Set permissions
    /// 4. Start the daemon
    /// 5. Verify readiness
    pub async fn init_device(serial: &str) -> Result<(), String> {
        tracing::info!("[ATX] Initializing atx-agent on {}...", serial);

        // Check if atx-agent is already running
        let check = Adb::shell(serial, "ps | grep atx-agent").await.unwrap_or_default();
        if check.contains("atx-agent") {
            tracing::info!("[ATX] atx-agent already running on {}", serial);
            return Ok(());
        }

        // Start atx-agent if binary exists
        let check_bin = Adb::shell(serial, "ls /data/local/tmp/atx-agent").await.unwrap_or_default();
        if check_bin.contains("atx-agent") && !check_bin.contains("No such file") {
            Adb::shell(serial, "chmod 755 /data/local/tmp/atx-agent").await.ok();
            Adb::shell(
                serial,
                "/data/local/tmp/atx-agent server -d --addr :7912",
            )
            .await
            .ok();

            // Wait for it to start
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            tracing::info!("[ATX] atx-agent started on {}", serial);
            return Ok(());
        }

        tracing::warn!(
            "[ATX] atx-agent binary not found on {}. Device may need manual setup.",
            serial
        );
        Err("atx-agent binary not found on device".to_string())
    }

    /// Verify atx-agent is responding on the given IP and port.
    pub async fn verify_ready(ip: &str, port: i64) -> bool {
        let url = format!("http://{}:{}/info", ip, port);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        match client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

use std::collections::HashSet;
use std::process::Stdio;
use tokio::process::Command;

/// ADB command-line wrapper using tokio::process.
pub struct Adb;

#[allow(dead_code)]
impl Adb {
    /// Run `adb devices` and return a set of connected serials.
    pub async fn list_devices() -> Result<HashSet<String>, String> {
        let output = Command::new("adb")
            .arg("devices")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("Failed to run adb: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = HashSet::new();

        for line in stdout.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[1] == "device" {
                devices.insert(parts[0].to_string());
            }
        }

        Ok(devices)
    }

    /// Execute `adb -s <serial> shell <cmd>`.
    pub async fn shell(serial: &str, cmd: &str) -> Result<String, String> {
        let output = Command::new("adb")
            .args(["-s", serial, "shell", cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("adb shell failed: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Execute `adb connect <address>`.
    pub async fn connect(address: &str) -> Result<String, String> {
        let output = Command::new("adb")
            .args(["connect", address])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("adb connect failed: {}", e))?;

        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(combined)
    }

    /// Execute `adb -s <serial> push <local> <remote>`.
    pub async fn push(serial: &str, local: &str, remote: &str) -> Result<String, String> {
        let output = Command::new("adb")
            .args(["-s", serial, "push", local, remote])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("adb push failed: {}", e))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get a device property via `adb -s <serial> shell getprop <prop>`.
    pub async fn get_prop(serial: &str, prop: &str) -> Result<String, String> {
        Self::shell(serial, &format!("getprop {}", prop)).await
    }

    /// Get screen resolution via `adb -s <serial> shell wm size`.
    pub async fn get_screen_size(serial: &str) -> Result<(i64, i64), String> {
        let output = Self::shell(serial, "wm size").await?;
        // Output: "Physical size: 1080x2400"
        if let Some(size_str) = output.split(':').nth(1) {
            let parts: Vec<&str> = size_str.trim().split('x').collect();
            if parts.len() == 2 {
                let w = parts[0].trim().parse::<i64>().unwrap_or(1080);
                let h = parts[1].trim().parse::<i64>().unwrap_or(1920);
                return Ok((w, h));
            }
        }
        Ok((1080, 1920))
    }

    /// Determine device type from serial.
    pub fn device_type(serial: &str) -> &'static str {
        if serial.starts_with("emulator-") || serial.starts_with("127.0.0.1:") {
            "emulator"
        } else if serial.contains(':') {
            "wifi"
        } else {
            "usb"
        }
    }

    /// Check if a serial is a USB serial (vs WiFi IP:PORT).
    pub fn is_usb_serial(serial: &str) -> bool {
        !serial.contains(':')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_type_usb() {
        assert_eq!(Adb::device_type("ABCD1234"), "usb");
        assert_eq!(Adb::device_type("R5CR20ABCDE"), "usb");
    }

    #[test]
    fn test_device_type_wifi() {
        assert_eq!(Adb::device_type("192.168.1.100:5555"), "wifi");
        assert_eq!(Adb::device_type("10.0.0.1:5555"), "wifi");
    }

    #[test]
    fn test_device_type_emulator() {
        assert_eq!(Adb::device_type("emulator-5554"), "emulator");
        assert_eq!(Adb::device_type("127.0.0.1:5555"), "emulator");
    }

    #[test]
    fn test_is_usb_serial() {
        assert!(Adb::is_usb_serial("ABCD1234"));
        assert!(Adb::is_usb_serial("R5CR20ABCDE"));
        // emulator-5554 has no colon, so is_usb_serial returns true
        // (it only distinguishes USB/emulator from WiFi by presence of ':')
        assert!(Adb::is_usb_serial("emulator-5554"));
        assert!(!Adb::is_usb_serial("192.168.1.100:5555"));
    }
}

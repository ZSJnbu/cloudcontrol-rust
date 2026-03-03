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

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("adb push failed (exit {}): {} {}", output.status, stderr, stdout));
        }

        let result = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(result)
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

    /// Tap at (x, y) via `adb -s <serial> shell input tap x y`.
    pub async fn input_tap(serial: &str, x: i32, y: i32) -> Result<(), String> {
        Self::shell(serial, &format!("input tap {} {}", x, y)).await?;
        Ok(())
    }

    /// Swipe via `adb -s <serial> shell input swipe x1 y1 x2 y2 duration_ms`.
    pub async fn input_swipe(serial: &str, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: i32) -> Result<(), String> {
        Self::shell(serial, &format!("input swipe {} {} {} {} {}", x1, y1, x2, y2, duration_ms)).await?;
        Ok(())
    }

    /// Input text via `adb -s <serial> shell input text <text>`.
    pub async fn input_text(serial: &str, text: &str) -> Result<(), String> {
        // Escape special characters for shell
        let escaped = text.replace('\\', "\\\\").replace(' ', "%s").replace('\"', "\\\"");
        Self::shell(serial, &format!("input text \"{}\"", escaped)).await?;
        Ok(())
    }

    /// Send keyevent via `adb -s <serial> shell input keyevent <keycode>`.
    pub async fn input_keyevent(serial: &str, keycode: &str) -> Result<(), String> {
        // Map common key names to Android keycodes
        let code = match keycode {
            "home" => "KEYCODE_HOME",
            "back" => "KEYCODE_BACK",
            "enter" => "KEYCODE_ENTER",
            "del" => "KEYCODE_DEL",
            "forward_del" => "KEYCODE_FORWARD_DEL",
            "tab" => "KEYCODE_TAB",
            "menu" => "KEYCODE_MENU",
            "power" => "KEYCODE_POWER",
            "wakeup" => "KEYCODE_WAKEUP",
            "dpad_up" => "KEYCODE_DPAD_UP",
            "dpad_down" => "KEYCODE_DPAD_DOWN",
            "dpad_left" => "KEYCODE_DPAD_LEFT",
            "dpad_right" => "KEYCODE_DPAD_RIGHT",
            other => other,
        };
        Self::shell(serial, &format!("input keyevent {}", code)).await?;
        Ok(())
    }

    /// Set up `adb -s <serial> forward tcp:0 tcp:<remote_port>` and return the assigned local port.
    pub async fn forward(serial: &str, remote_port: u16) -> Result<u16, String> {
        let output = Command::new("adb")
            .args([
                "-s",
                serial,
                "forward",
                "tcp:0",
                &format!("tcp:{}", remote_port),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("adb forward failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        // adb forward tcp:0 returns the assigned port number on stdout
        if let Ok(port) = stdout.parse::<u16>() {
            return Ok(port);
        }

        // Some adb versions output port on stderr or don't output it at all.
        // Try to parse from stderr.
        if let Ok(port) = stderr.parse::<u16>() {
            return Ok(port);
        }

        // Fallback: list forwards and find the one we just created
        let list_output = Command::new("adb")
            .args(["-s", serial, "forward", "--list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("adb forward --list failed: {}", e))?;

        let list_stdout = String::from_utf8_lossy(&list_output.stdout);
        let remote_target = format!("tcp:{}", remote_port);

        // Find the last forward entry matching our remote port
        for line in list_stdout.lines().rev() {
            // Format: "serial tcp:LOCAL_PORT tcp:REMOTE_PORT"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == serial && parts[2] == remote_target {
                if let Some(port_str) = parts[1].strip_prefix("tcp:") {
                    if let Ok(port) = port_str.parse::<u16>() {
                        return Ok(port);
                    }
                }
            }
        }

        Err(format!(
            "Failed to determine forwarded port. stdout={}, stderr={}",
            stdout, stderr
        ))
    }

    /// Take a screenshot via `adb -s <serial> exec-out screencap -p` → returns PNG bytes.
    /// This is a fallback when u2 server is not available.
    pub async fn screencap(serial: &str) -> Result<Vec<u8>, String> {
        let output = Command::new("adb")
            .args(["-s", serial, "exec-out", "screencap", "-p"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("adb screencap failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("adb screencap error: {}", stderr));
        }

        if output.stdout.is_empty() {
            return Err("adb screencap returned empty data".to_string());
        }

        Ok(output.stdout)
    }

    /// Set up `adb -s <serial> forward tcp:0 localabstract:<name>` and return the assigned local port.
    pub async fn forward_abstract(serial: &str, name: &str) -> Result<u16, String> {
        let output = Command::new("adb")
            .args([
                "-s",
                serial,
                "forward",
                "tcp:0",
                &format!("localabstract:{}", name),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("adb forward abstract failed: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if let Ok(port) = stdout.parse::<u16>() {
            return Ok(port);
        }
        if let Ok(port) = stderr.parse::<u16>() {
            return Ok(port);
        }

        Err(format!(
            "Failed to determine forwarded port for abstract:{}. stdout={}, stderr={}",
            name, stdout, stderr
        ))
    }

    /// Remove a port forwarding: `adb -s <serial> forward --remove tcp:<local_port>`.
    pub async fn forward_remove(serial: &str, local_port: u16) -> Result<(), String> {
        Command::new("adb")
            .args([
                "-s",
                serial,
                "forward",
                "--remove",
                &format!("tcp:{}", local_port),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("adb forward --remove failed: {}", e))?;
        Ok(())
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

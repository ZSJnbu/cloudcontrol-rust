use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Device model matching the SQLite `devices` table.
/// JSON output uses MongoDB-style field names for API compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct Device {
    pub udid: String,
    #[serde(default)]
    pub serial: Option<String>,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub port: Option<i64>,
    #[serde(default)]
    pub present: bool,
    #[serde(default)]
    pub ready: bool,
    /// JSON output: "using" (mapped from DB column "using_device")
    #[serde(rename = "using", default)]
    pub using_device: bool,
    #[serde(default)]
    pub is_server: bool,
    #[serde(default)]
    pub is_mock: bool,
    #[serde(default)]
    pub update_time: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub sdk: Option<i64>,
    #[serde(default)]
    pub memory: Option<Value>,
    #[serde(default)]
    pub cpu: Option<Value>,
    #[serde(default)]
    pub battery: Option<Value>,
    #[serde(default)]
    pub display: Option<Value>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    /// JSON output: "agentVersion" (mapped from DB column "agent_version")
    #[serde(rename = "agentVersion", default)]
    pub agent_version: Option<String>,
    #[serde(default)]
    pub hwaddr: Option<String>,
    /// JSON output: "createdAt" (mapped from DB column "created_at")
    #[serde(rename = "createdAt", default)]
    pub created_at: Option<String>,
    /// JSON output: "updatedAt" (mapped from DB column "updated_at")
    #[serde(rename = "updatedAt", default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub extra_data: Option<Value>,
}

#[allow(dead_code)]
impl Device {
    /// Build a Device from a HashMap (MongoDB-style keys as used by service layer)
    pub fn from_map(map: &serde_json::Map<String, Value>) -> Self {
        let get_str = |k: &str| map.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
        let get_i64 = |k: &str| map.get(k).and_then(|v| v.as_i64());
        let get_bool = |k: &str| map.get(k).and_then(|v| v.as_bool()).unwrap_or(false);

        Device {
            udid: get_str("udid").unwrap_or_default(),
            serial: get_str("serial"),
            ip: get_str("ip"),
            port: get_i64("port"),
            present: get_bool("present"),
            ready: get_bool("ready"),
            using_device: get_bool("using"),
            is_server: get_bool("is_server"),
            is_mock: get_bool("is_mock"),
            update_time: get_str("update_time"),
            model: get_str("model"),
            brand: get_str("brand"),
            version: get_str("version"),
            sdk: get_i64("sdk"),
            memory: map.get("memory").cloned(),
            cpu: map.get("cpu").cloned(),
            battery: map.get("battery").cloned(),
            display: map.get("display").cloned(),
            owner: get_str("owner"),
            provider: get_str("provider"),
            agent_version: get_str("agentVersion").or_else(|| get_str("agent_version")),
            hwaddr: get_str("hwaddr"),
            created_at: get_str("createdAt").or_else(|| get_str("created_at")),
            updated_at: get_str("updatedAt").or_else(|| get_str("updated_at")),
            extra_data: map.get("extra_data").cloned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_default() {
        let d = Device::default();
        assert_eq!(d.udid, "");
        assert!(!d.present);
        assert!(!d.ready);
        assert!(!d.using_device);
        assert!(!d.is_server);
        assert!(!d.is_mock);
        assert!(d.serial.is_none());
        assert!(d.ip.is_none());
        assert!(d.model.is_none());
        assert!(d.memory.is_none());
    }

    #[test]
    fn test_device_from_map_complete() {
        let map: serde_json::Map<String, Value> = serde_json::from_str(r#"{
            "udid": "test-device-1",
            "serial": "ABC123",
            "ip": "192.168.1.100",
            "port": 7912,
            "present": true,
            "ready": true,
            "using": false,
            "is_mock": false,
            "model": "Pixel 5",
            "brand": "Google",
            "version": "12",
            "sdk": 31,
            "memory": {"total": 8192},
            "agentVersion": "0.10.0",
            "createdAt": "2024-01-01"
        }"#).unwrap();

        let d = Device::from_map(&map);
        assert_eq!(d.udid, "test-device-1");
        assert_eq!(d.serial, Some("ABC123".into()));
        assert_eq!(d.ip, Some("192.168.1.100".into()));
        assert_eq!(d.port, Some(7912));
        assert!(d.present);
        assert!(d.ready);
        assert!(!d.using_device);
        assert_eq!(d.model, Some("Pixel 5".into()));
        assert_eq!(d.brand, Some("Google".into()));
        assert_eq!(d.sdk, Some(31));
        assert_eq!(d.agent_version, Some("0.10.0".into()));
        assert_eq!(d.created_at, Some("2024-01-01".into()));
        assert!(d.memory.is_some());
    }

    #[test]
    fn test_device_from_map_minimal() {
        let map: serde_json::Map<String, Value> = serde_json::from_str(r#"{
            "udid": "minimal-device"
        }"#).unwrap();

        let d = Device::from_map(&map);
        assert_eq!(d.udid, "minimal-device");
        assert!(!d.present);
        assert!(d.serial.is_none());
        assert!(d.model.is_none());
    }

    #[test]
    fn test_device_serde_rename() {
        let d = Device {
            udid: "test".into(),
            using_device: true,
            agent_version: Some("1.0".into()),
            created_at: Some("2024-01-01".into()),
            updated_at: Some("2024-01-02".into()),
            ..Default::default()
        };

        let json = serde_json::to_value(&d).unwrap();
        // using_device should be serialized as "using"
        assert_eq!(json["using"], true);
        assert!(json.get("using_device").is_none());
        // agent_version → agentVersion
        assert_eq!(json["agentVersion"], "1.0");
        // created_at → createdAt
        assert_eq!(json["createdAt"], "2024-01-01");
        // updated_at → updatedAt
        assert_eq!(json["updatedAt"], "2024-01-02");
    }

    #[test]
    fn test_device_json_fields() {
        let d = Device {
            udid: "test".into(),
            memory: Some(serde_json::json!({"total": 8192, "free": 4096})),
            cpu: Some(serde_json::json!({"cores": 8})),
            battery: Some(serde_json::json!({"level": 85})),
            display: Some(serde_json::json!({"width": 1080, "height": 1920})),
            ..Default::default()
        };

        let json = serde_json::to_value(&d).unwrap();
        assert_eq!(json["memory"]["total"], 8192);
        assert_eq!(json["cpu"]["cores"], 8);
        assert_eq!(json["battery"]["level"], 85);
        assert_eq!(json["display"]["width"], 1080);
    }
}

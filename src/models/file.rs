use serde::{Deserialize, Serialize};
use serde_json::Value;

/// InstalledFile model matching the SQLite `installed_file` table.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct InstalledFile {
    #[serde(rename = "group", default)]
    pub group_name: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub filesize: Option<i64>,
    #[serde(default)]
    pub upload_time: Option<String>,
    #[serde(default)]
    pub who: Option<String>,
    #[serde(default)]
    pub extra_data: Option<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installed_file_serde_rename() {
        let f = InstalledFile {
            group_name: "test_group".into(),
            filename: "test.apk".into(),
            filesize: Some(1024),
            ..Default::default()
        };
        let json = serde_json::to_value(&f).unwrap();
        // group_name should be serialized as "group"
        assert_eq!(json["group"], "test_group");
        assert!(json.get("group_name").is_none());
        assert_eq!(json["filename"], "test.apk");
        assert_eq!(json["filesize"], 1024);
    }

    #[test]
    fn test_installed_file_deserialize() {
        let json_str = r#"{"group":"mygroup","filename":"app.apk","filesize":2048,"upload_time":"2024-01-01","who":"admin"}"#;
        let f: InstalledFile = serde_json::from_str(json_str).unwrap();
        assert_eq!(f.group_name, "mygroup");
        assert_eq!(f.filename, "app.apk");
        assert_eq!(f.filesize, Some(2048));
        assert_eq!(f.upload_time, Some("2024-01-01".into()));
        assert_eq!(f.who, Some("admin".into()));
    }

    #[test]
    fn test_installed_file_defaults() {
        let f = InstalledFile::default();
        assert_eq!(f.group_name, "");
        assert_eq!(f.filename, "");
        assert!(f.filesize.is_none());
        assert!(f.upload_time.is_none());
        assert!(f.who.is_none());
        assert!(f.extra_data.is_none());
    }
}

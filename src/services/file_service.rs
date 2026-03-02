use crate::db::Database;
use serde_json::Value;

/// File management CRUD proxy — replaces Python `file_service_impl.py`.
#[derive(Clone)]
pub struct FileService {
    db: Database,
}

impl FileService {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn save_install_file(&self, file: &Value) -> Result<(), String> {
        let group = file
            .get("group")
            .and_then(|v| v.as_str())
            .or_else(|| file.get("group").and_then(|v| v.as_i64()).map(|_| "0"))
            .unwrap_or("");
        let filename = file.get("filename").and_then(|v| v.as_str()).unwrap_or("");
        let filesize = file.get("filesize").and_then(|v| v.as_i64());
        let upload_time = file
            .get("upload_time")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let who = file.get("who").and_then(|v| v.as_str()).unwrap_or("");

        // Collect extra data
        let known = ["group", "filename", "filesize", "upload_time", "who"];
        let extra: serde_json::Map<String, Value> = file
            .as_object()
            .map(|o| {
                o.iter()
                    .filter(|(k, _)| !known.contains(&k.as_str()))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let extra_str = if extra.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&extra).unwrap_or_default())
        };

        // Handle numeric group
        let group_str = if group.is_empty() {
            file.get("group")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string())
                .unwrap_or_default()
        } else {
            group.to_string()
        };

        self.db
            .save_install_file(
                &group_str,
                filename,
                filesize,
                upload_time,
                who,
                extra_str.as_deref(),
            )
            .await
            .map_err(|e| format!("Save file failed: {}", e))
    }

    pub async fn query_install_file(
        &self,
        group: &str,
        start: i64,
        limit: i64,
        _sort: &str,
    ) -> Result<Vec<Value>, String> {
        self.db
            .query_install_file(group, start, limit)
            .await
            .map_err(|e| format!("Query files failed: {}", e))
    }

    pub async fn query_all_install_file(&self) -> Result<i64, String> {
        self.db
            .query_all_install_file()
            .await
            .map_err(|e| format!("Count files failed: {}", e))
    }

    pub async fn delete_install_file(&self, group: &str, filename: &str) -> Result<(), String> {
        self.db
            .delete_install_file(group, filename)
            .await
            .map_err(|e| format!("Delete file failed: {}", e))
    }
}

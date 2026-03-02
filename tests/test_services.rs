mod common;

use cloudcontrol::services::file_service::FileService;
use cloudcontrol::services::phone_service::PhoneService;
use common::{create_temp_db, make_device_json};
use serde_json::json;

#[tokio::test]
async fn test_phone_service_update_field() {
    let (_tmp, db) = create_temp_db().await;
    let svc = PhoneService::new(db);

    let data = make_device_json("ps-dev-1", true, false);
    svc.update_field("ps-dev-1", &data).await.unwrap();

    let result = svc.query_info_by_udid("ps-dev-1").await.unwrap();
    assert!(result.is_some());
    let device = result.unwrap();
    assert_eq!(device["udid"], "ps-dev-1");
    assert_eq!(device["model"], "TestPhone");
    assert_eq!(device["present"], true);
}

#[tokio::test]
async fn test_phone_service_offline() {
    let (_tmp, db) = create_temp_db().await;
    let svc = PhoneService::new(db);

    // Register device
    let data = make_device_json("offline-dev", true, false);
    svc.update_field("offline-dev", &data).await.unwrap();

    // Mark offline
    svc.offline_connected("offline-dev").await.unwrap();

    let result = svc.query_info_by_udid("offline-dev").await.unwrap().unwrap();
    assert_eq!(result["present"], false);
}

#[tokio::test]
async fn test_phone_service_query_device_list() {
    let (_tmp, db) = create_temp_db().await;
    let svc = PhoneService::new(db);

    // Insert 3 devices, 2 online, 1 offline
    svc.update_field("dev-online-1", &make_device_json("dev-online-1", true, false))
        .await
        .unwrap();
    svc.update_field("dev-online-2", &make_device_json("dev-online-2", true, false))
        .await
        .unwrap();
    svc.update_field("dev-offline", &make_device_json("dev-offline", false, false))
        .await
        .unwrap();

    // query_device_list returns only present=true devices
    let list = svc.query_device_list().await.unwrap();
    assert_eq!(list.len(), 2);

    let udids: Vec<&str> = list
        .iter()
        .map(|d| d["udid"].as_str().unwrap())
        .collect();
    assert!(udids.contains(&"dev-online-1"));
    assert!(udids.contains(&"dev-online-2"));
    assert!(!udids.contains(&"dev-offline"));
}

#[tokio::test]
async fn test_phone_service_delete_all() {
    let (_tmp, db) = create_temp_db().await;
    let svc = PhoneService::new(db);

    svc.update_field("del-dev-1", &make_device_json("del-dev-1", true, false))
        .await
        .unwrap();
    svc.update_field("del-dev-2", &make_device_json("del-dev-2", true, false))
        .await
        .unwrap();

    svc.delete_devices().await.unwrap();

    let list = svc.query_device_list().await.unwrap();
    assert_eq!(list.len(), 0);
}

#[tokio::test]
async fn test_file_service_save_and_query() {
    let (_tmp, db) = create_temp_db().await;
    let svc = FileService::new(db);

    let file_data = json!({
        "group": "0",
        "filename": "test.apk",
        "filesize": 2048,
        "upload_time": "2024-01-01 12:00:00",
        "who": "admin",
    });
    svc.save_install_file(&file_data).await.unwrap();

    let files = svc.query_install_file("0", 0, 10, "").await.unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["filename"], "test.apk");
}

#[tokio::test]
async fn test_file_service_count() {
    let (_tmp, db) = create_temp_db().await;
    let svc = FileService::new(db);

    for i in 0..5 {
        let file_data = json!({
            "group": "0",
            "filename": format!("file_{}.apk", i),
            "filesize": 1000 + i,
            "upload_time": "2024-01-01",
            "who": "user",
        });
        svc.save_install_file(&file_data).await.unwrap();
    }

    let count = svc.query_all_install_file().await.unwrap();
    assert_eq!(count, 5);
}

#[tokio::test]
async fn test_file_service_delete() {
    let (_tmp, db) = create_temp_db().await;
    let svc = FileService::new(db);

    let file_data = json!({
        "group": "0",
        "filename": "to_delete.apk",
        "filesize": 512,
        "upload_time": "2024-01-01",
        "who": "admin",
    });
    svc.save_install_file(&file_data).await.unwrap();

    svc.delete_install_file("0", "to_delete.apk").await.unwrap();

    let files = svc.query_install_file("0", 0, 10, "").await.unwrap();
    assert_eq!(files.len(), 0);
}

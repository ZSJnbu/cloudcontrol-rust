use crate::device::adb::Adb;
use crate::device::scrcpy::ScrcpySession;
use crate::services::scrcpy_manager::ScrcpyManager;
use crate::state::AppState;
use actix_web::{web, HttpRequest, HttpResponse};
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// GET /scrcpy/{udid}/ws → Binary WebSocket for scrcpy video + control
pub async fn scrcpy_websocket(
    state: web::Data<AppState>,
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
) -> HttpResponse {
    let udid = path.into_inner();
    if udid.is_empty() {
        return HttpResponse::BadRequest().body("Missing udid");
    }

    let (resp, mut session, mut msg_stream) = match actix_ws::handle(&req, stream) {
        Ok(v) => v,
        Err(e) => return HttpResponse::InternalServerError().body(format!("WS error: {}", e)),
    };

    let state = state.into_inner().clone();
    let udid_clone = udid.clone();

    actix_web::rt::spawn(async move {
        // Look up device serial from DB
        let phone_service =
            crate::services::phone_service::PhoneService::new(state.db.clone());
        let device = match phone_service.query_info_by_udid(&udid_clone).await {
            Ok(Some(d)) => d,
            _ => {
                let _ = session
                    .text(
                        serde_json::to_string(
                            &json!({"type":"error","message":"Device not found"}),
                        )
                        .unwrap(),
                    )
                    .await;
                let _ = session.close(None).await;
                return;
            }
        };

        let serial = device
            .get("serial")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if serial.is_empty() {
            let _ = session
                .text(
                    serde_json::to_string(
                        &json!({"type":"error","message":"Device serial not found"}),
                    )
                    .unwrap(),
                )
                .await;
            let _ = session.close(None).await;
            return;
        }

        tracing::info!("[Scrcpy WS] Starting session for {} ({})", udid_clone, serial);

        // Start scrcpy session
        let scrcpy = match ScrcpySession::start(&serial).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("[Scrcpy WS] Failed to start scrcpy for {}: {}", serial, e);
                let _ = session
                    .text(
                        serde_json::to_string(
                            &json!({"type":"error","message": format!("scrcpy start failed: {}", e)}),
                        )
                        .unwrap(),
                    )
                    .await;
                let _ = session.close(None).await;
                return;
            }
        };

        let meta = scrcpy.meta.clone();

        // Send init message with codec and dimensions
        let init_msg = json!({
            "type": "init",
            "codec": "h264",
            "width": meta.width,
            "height": meta.height,
            "deviceName": meta.device_name,
        });
        if session
            .text(serde_json::to_string(&init_msg).unwrap())
            .await
            .is_err()
        {
            scrcpy.shutdown().await;
            return;
        }

        tracing::info!(
            "[Scrcpy WS] Session active: {} ({}x{})",
            serial,
            meta.width,
            meta.height
        );

        // Split scrcpy session for concurrent video read + control write
        let scrcpy = Arc::new(Mutex::new(scrcpy));
        let scrcpy_video = scrcpy.clone();
        let scrcpy_control = scrcpy.clone();

        let session_clone = session.clone();
        let serial_clone = serial.clone();

        // Video task: read frames and send as binary WS messages
        let video_task = tokio::spawn(async move {
            let mut session = session_clone;
            loop {
                let frame = {
                    let mut s = scrcpy_video.lock().await;
                    s.read_frame().await
                };

                match frame {
                    Ok(frame) => {
                        // Binary message format:
                        // flags (1 byte): bit0 = config, bit1 = keyframe
                        // size (4 bytes BE): NAL data size
                        // data: H.264 NAL units
                        let flags: u8 =
                            (if frame.is_config { 1 } else { 0 }) | (if frame.is_key { 2 } else { 0 });
                        let size = (frame.data.len() as u32).to_be_bytes();

                        let mut msg = Vec::with_capacity(5 + frame.data.len());
                        msg.push(flags);
                        msg.extend_from_slice(&size);
                        msg.extend_from_slice(&frame.data);

                        if session.binary(msg).await.is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("[Scrcpy WS] Video read error for {}: {}", serial_clone, e);
                        break;
                    }
                }
            }
        });

        // Control receive: browser WS binary → scrcpy control socket
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                actix_ws::Message::Binary(data) => {
                    let mut s = scrcpy_control.lock().await;
                    // Forward raw binary directly to scrcpy control socket
                    if data.len() >= 2 {
                        let msg_type = data[0];
                        match msg_type {
                            // Touch event from browser (28 bytes)
                            2 => {
                                if data.len() >= 28 {
                                    if let Err(e) = s.send_touch(
                                        data[1],
                                        u32::from_be_bytes(data[10..14].try_into().unwrap()),
                                        u32::from_be_bytes(data[14..18].try_into().unwrap()),
                                        u16::from_be_bytes(data[18..20].try_into().unwrap()),
                                        u16::from_be_bytes(data[20..22].try_into().unwrap()),
                                        u16::from_be_bytes(data[22..24].try_into().unwrap()),
                                    ).await {
                                        tracing::warn!("[Scrcpy WS] Touch send error: {}", e);
                                    }
                                }
                            }
                            // Key event from browser (14 bytes)
                            0 => {
                                if data.len() >= 14 {
                                    if let Err(e) = s.send_key(
                                        data[1],
                                        u32::from_be_bytes(data[2..6].try_into().unwrap()),
                                    ).await {
                                        tracing::warn!("[Scrcpy WS] Key send error: {}", e);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                actix_ws::Message::Close(_) => break,
                _ => {}
            }
        }

        // Cleanup
        video_task.abort();
        {
            let mut s = scrcpy.lock().await;
            let _ = s.video_stream.shutdown().await;
            let _ = s.control_stream.shutdown().await;
            if let Some(mut proc) = s.server_process.take() {
                let _ = proc.kill().await;
            }
            let _ = Adb::forward_remove(&s.serial, s.local_port).await;
        }

        tracing::info!("[Scrcpy WS] Session closed for {}", udid_clone);
    });

    resp
}

/// GET /scrcpy/{udid}/status → check if scrcpy is available for this device
pub async fn scrcpy_status(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let udid = path.into_inner();

    let phone_service =
        crate::services::phone_service::PhoneService::new(state.db.clone());
    let device = match phone_service.query_info_by_udid(&udid).await {
        Ok(Some(d)) => d,
        _ => {
            return HttpResponse::Ok().json(json!({"available": false, "reason": "device not found"}));
        }
    };

    let serial = device
        .get("serial")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if serial.is_empty() {
        return HttpResponse::Ok().json(json!({"available": false, "reason": "no serial"}));
    }

    // Check if JAR is available locally
    let jar_available = ScrcpyManager::jar_available();

    HttpResponse::Ok().json(json!({
        "available": jar_available,
        "serial": serial,
    }))
}

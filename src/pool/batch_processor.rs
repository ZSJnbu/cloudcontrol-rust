#![allow(dead_code)]

use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

/// A submitted batch event.
struct BatchItem {
    event_type: String,
    data: Value,
    reply: oneshot::Sender<Result<Value, String>>,
}

type Handler = Box<dyn Fn(Value) -> futures::future::BoxFuture<'static, Result<Value, String>> + Send + Sync>;

/// Async batch processor — collects events and processes them in batches.
/// Replaces Python `AsyncBatchProcessor` in aio_pool.py.
pub struct BatchProcessor {
    tx: mpsc::Sender<BatchItem>,
    handle: Option<JoinHandle<()>>,
}

impl BatchProcessor {
    pub fn new(
        batch_size: usize,
        flush_interval: Duration,
        handlers: HashMap<String, Handler>,
    ) -> Self {
        let (tx, rx) = mpsc::channel(1000);

        let handle = tokio::spawn(Self::process_loop(rx, batch_size, flush_interval, handlers));

        Self {
            tx,
            handle: Some(handle),
        }
    }

    /// Submit an event for batch processing.
    pub async fn submit(
        &self,
        event_type: &str,
        data: Value,
    ) -> Result<Value, String> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.tx
            .send(BatchItem {
                event_type: event_type.to_string(),
                data,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "Batch processor channel closed".to_string())?;

        reply_rx
            .await
            .map_err(|_| "Batch processor reply dropped".to_string())?
    }

    async fn process_loop(
        mut rx: mpsc::Receiver<BatchItem>,
        batch_size: usize,
        flush_interval: Duration,
        handlers: HashMap<String, Handler>,
    ) {
        loop {
            let mut batch = Vec::new();
            let deadline = tokio::time::Instant::now() + flush_interval;

            // Collect items
            loop {
                if batch.len() >= batch_size {
                    break;
                }
                let timeout = deadline.saturating_duration_since(tokio::time::Instant::now());
                match tokio::time::timeout(timeout, rx.recv()).await {
                    Ok(Some(item)) => batch.push(item),
                    Ok(None) => return, // channel closed
                    Err(_) => break,    // timeout
                }
            }

            if batch.is_empty() {
                continue;
            }

            // Group by event type
            let mut groups: HashMap<String, Vec<BatchItem>> = HashMap::new();
            for item in batch {
                groups
                    .entry(item.event_type.clone())
                    .or_default()
                    .push(item);
            }

            // Process each group
            for (event_type, items) in groups {
                if let Some(handler) = handlers.get(&event_type) {
                    for item in items {
                        let result = handler(item.data).await;
                        let _ = item.reply.send(result);
                    }
                } else {
                    for item in items {
                        let _ = item
                            .reply
                            .send(Err(format!("No handler for event: {}", event_type)));
                    }
                }
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_submit_with_handler() {
        let mut handlers: HashMap<String, Handler> = HashMap::new();
        handlers.insert(
            "echo".to_string(),
            Box::new(|data| {
                Box::pin(async move { Ok(data) })
            }),
        );

        let processor = BatchProcessor::new(10, Duration::from_millis(100), handlers);
        let result = processor.submit("echo", json!({"msg": "hello"})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap()["msg"], "hello");
    }

    #[tokio::test]
    async fn test_submit_no_handler() {
        let handlers: HashMap<String, Handler> = HashMap::new();
        let processor = BatchProcessor::new(10, Duration::from_millis(100), handlers);
        let result = processor.submit("unknown_event", json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No handler"));
    }

    #[tokio::test]
    async fn test_batch_grouping() {
        let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let mut handlers: HashMap<String, Handler> = HashMap::new();
        handlers.insert(
            "count".to_string(),
            Box::new(move |_data| {
                let c = counter_clone.clone();
                Box::pin(async move {
                    c.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(json!({"ok": true}))
                })
            }),
        );

        let processor = BatchProcessor::new(10, Duration::from_millis(50), handlers);

        // Submit 3 events of the same type
        let (r1, r2, r3) = tokio::join!(
            processor.submit("count", json!({})),
            processor.submit("count", json!({})),
            processor.submit("count", json!({})),
        );
        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(r3.is_ok());
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    }
}

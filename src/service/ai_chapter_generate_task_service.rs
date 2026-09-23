use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::RwLock;

use crate::util::time::now_ts;

const FINISHED_TASK_RETENTION: Duration = Duration::from_secs(30 * 60);
const RUNNING_TASK_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// Background single-chapter AI generation task.
///
/// Long-running synchronous AI requests can be cut off by reverse proxies
/// (nginx ~60s, Cloudflare ~100s) with a 504 before the model finishes, so the
/// frontend starts a task and polls its status instead of holding one request
/// open.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterGenerateTaskView {
    pub status: String,
    pub error: Option<String>,
    pub result: Option<serde_json::Value>,
    pub started_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Default)]
pub struct AiChapterGenerateTaskService {
    tasks: Arc<RwLock<HashMap<String, ChapterGenerateTaskView>>>,
}

impl AiChapterGenerateTaskService {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start<F, Fut>(&self, key: String, task: F) -> ChapterGenerateTaskView
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static,
    {
        self.cleanup_finished().await;

        let mut tasks = self.tasks.write().await;
        if let Some(existing) = tasks.get(&key) {
            if existing.status == "running" {
                return existing.clone();
            }
        }
        let now = now_ts() * 1000;
        let view = ChapterGenerateTaskView {
            status: "running".to_string(),
            error: None,
            result: None,
            started_at: now,
            updated_at: now,
        };
        tasks.insert(key.clone(), view.clone());
        drop(tasks);

        let service = self.clone();
        tokio::spawn(async move {
            let result = tokio::time::timeout(RUNNING_TASK_TIMEOUT, task()).await;
            let result = match result {
                Ok(inner) => inner,
                Err(_) => Err(format!(
                    "任务超时（{}秒），已自动终止",
                    RUNNING_TASK_TIMEOUT.as_secs()
                )),
            };
            let mut tasks = service.tasks.write().await;
            if let Some(task) = tasks.get_mut(&key) {
                match result {
                    Ok(value) => {
                        task.status = "completed".to_string();
                        task.error = None;
                        task.result = Some(value);
                    }
                    Err(error) => {
                        task.status = "failed".to_string();
                        task.error = Some(error);
                        task.result = None;
                    }
                }
                task.updated_at = now_ts() * 1000;
            }
        });
        view
    }

    pub async fn get(&self, key: &str) -> Option<ChapterGenerateTaskView> {
        self.tasks.read().await.get(key).cloned()
    }

    async fn cleanup_finished(&self) {
        let now = now_ts() * 1000;
        let finished_cutoff = now - FINISHED_TASK_RETENTION.as_millis() as i64;
        let running_cutoff = now - RUNNING_TASK_TIMEOUT.as_millis() as i64;
        let mut tasks = self.tasks.write().await;
        tasks.retain(|_, task| {
            if task.status == "running" {
                // 清理超过超时时间仍在 running 的卡死任务
                task.updated_at >= running_cutoff
            } else {
                task.updated_at >= finished_cutoff
            }
        });
    }
}

pub fn chapter_generate_task_key(user_ns: &str, book_url: &str, chapter_index: i32) -> String {
    format!("chapter-generate::{user_ns}::{book_url}::{chapter_index}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn start_returns_running_then_completed() {
        let service = AiChapterGenerateTaskService::new();
        let view = service
            .start("k".to_string(), || async {
                Ok(serde_json::json!({ "ok": true }))
            })
            .await;
        assert_eq!(view.status, "running");

        for _ in 0..50 {
            let view = service.get("k").await.unwrap();
            if view.status == "completed" {
                assert_eq!(view.result.unwrap()["ok"], serde_json::json!(true));
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("task did not complete in time");
    }

    #[tokio::test]
    async fn failed_task_records_error() {
        let service = AiChapterGenerateTaskService::new();
        service
            .start("k".to_string(), || async { Err("boom".to_string()) })
            .await;
        for _ in 0..50 {
            let view = service.get("k").await.unwrap();
            if view.status == "failed" {
                assert_eq!(view.error.as_deref(), Some("boom"));
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("task did not fail in time");
    }

    #[tokio::test]
    async fn start_while_running_returns_existing_task() {
        let service = AiChapterGenerateTaskService::new();
        let first = service
            .start("k".to_string(), || async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(serde_json::json!({}))
            })
            .await;
        let second = service
            .start("k".to_string(), || async {
                Ok(serde_json::json!({ "other": true }))
            })
            .await;
        assert_eq!(second.started_at, first.started_at);
    }

    #[tokio::test]
    async fn stale_running_task_is_cleaned_up() {
        let service = AiChapterGenerateTaskService::new();
        {
            let mut tasks = service.tasks.write().await;
            let stale_time = (now_ts() * 1000) - (RUNNING_TASK_TIMEOUT.as_millis() as i64) - 1000;
            tasks.insert(
                "stale".to_string(),
                ChapterGenerateTaskView {
                    status: "running".to_string(),
                    error: None,
                    result: None,
                    started_at: stale_time,
                    updated_at: stale_time,
                },
            );
        }
        service.cleanup_finished().await;
        assert!(service.get("stale").await.is_none());
    }

    #[tokio::test]
    async fn recent_running_task_is_not_cleaned_up() {
        let service = AiChapterGenerateTaskService::new();
        {
            let mut tasks = service.tasks.write().await;
            let now = now_ts() * 1000;
            tasks.insert(
                "active".to_string(),
                ChapterGenerateTaskView {
                    status: "running".to_string(),
                    error: None,
                    result: None,
                    started_at: now,
                    updated_at: now,
                },
            );
        }
        service.cleanup_finished().await;
        assert!(service.get("active").await.is_some());
    }
}

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::util::time::now_ts;

/// 已完成任务视图的保留时长，超过后从内存中清理。
const FINISHED_TASK_RETENTION: Duration = Duration::from_secs(30 * 60);
/// 运行中任务的最长执行时长，超时任务按失败记录。
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
    /// 每次运行的唯一标识；迟到的旧 worker 只能写回自己那一次运行的视图。
    #[serde(skip)]
    pub run_id: String,
}

/// 章节生成后台任务注册表，支持启动、查询与过期清理。
#[derive(Clone, Default)]
pub struct AiChapterGenerateTaskService {
    tasks: Arc<RwLock<HashMap<String, ChapterGenerateTaskView>>>,
}

impl AiChapterGenerateTaskService {
    /// 创建空的任务服务。
    pub fn new() -> Self {
        Self::default()
    }

    /// 启动后台任务并立即返回运行中视图。
    ///
    /// 相同 key 已有运行中任务时直接复用；实际执行受 [`RUNNING_TASK_TIMEOUT`] 约束，
    /// 超时按失败记录。每次运行携带唯一 `run_id`，防止被清理/重试替换后旧 worker 写回结果。
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
        let run_id = Uuid::new_v4().to_string();
        let view = ChapterGenerateTaskView {
            status: "running".to_string(),
            error: None,
            result: None,
            started_at: now,
            updated_at: now,
            run_id: run_id.clone(),
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
            service.finish(&key, &run_id, result).await;
        });
        view
    }

    /// 记录任务运行结果。
    ///
    /// 仅当目标任务视图仍是本次 `run_id` 的运行中任务时才写入；视图被清理、
    /// 被同 key 的新任务替换或已经写回过时，丢弃迟到的旧 worker 结果。
    async fn finish(&self, key: &str, run_id: &str, result: Result<serde_json::Value, String>) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(key) {
            if task.run_id != run_id || task.status != "running" {
                return;
            }
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
    }

    /// 查询任务当前视图；任务不存在时返回 `None`。
    pub async fn get(&self, key: &str) -> Option<ChapterGenerateTaskView> {
        self.tasks.read().await.get(key).cloned()
    }

    /// 清理超期的已完成任务与长时间卡在运行中的僵尸任务。
    ///
    /// 即使僵尸视图被清理后同 key 被复用，旧 worker 也会因 `run_id` 不匹配而无法写回。
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

/// 构造章节生成后台任务的唯一 key。
pub fn chapter_generate_task_key(user_ns: &str, book_url: &str, chapter_index: i32) -> String {
    format!("chapter-generate::{user_ns}::{book_url}::{chapter_index}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证任务从运行中推进到完成并记录结果。
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

    /// 验证失败任务记录错误信息。
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

    /// 验证运行中的同 key 任务被复用而不是重复启动。
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

    /// 验证超期的运行中僵尸任务会被清理。
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
                    run_id: String::new(),
                },
            );
        }
        service.cleanup_finished().await;
        assert!(service.get("stale").await.is_none());
    }

    /// 验证未超期的运行中任务不会被清理。
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
                    run_id: String::new(),
                },
            );
        }
        service.cleanup_finished().await;
        assert!(service.get("active").await.is_some());
    }

    /// 验证迟到的旧 worker 不能覆盖同 key 新任务的结果。
    #[tokio::test]
    async fn stale_worker_cannot_overwrite_restarted_task() {
        let service = AiChapterGenerateTaskService::new();
        let old = service
            .start("k".to_string(), || async { Err("old".to_string()) })
            .await;
        // 模拟旧视图被清理后同 key 重新启动，新运行使用新的 run_id。
        {
            let mut tasks = service.tasks.write().await;
            let now = now_ts() * 1000;
            tasks.insert(
                "k".to_string(),
                ChapterGenerateTaskView {
                    status: "running".to_string(),
                    error: None,
                    result: None,
                    started_at: now,
                    updated_at: now,
                    run_id: "new-run".to_string(),
                },
            );
        }
        service
            .finish("k", &old.run_id, Ok(serde_json::json!({ "old": true })))
            .await;
        let view = service.get("k").await.unwrap();
        assert_eq!(view.status, "running");
        assert_eq!(view.run_id, "new-run");
        assert!(view.result.is_none());
    }
}

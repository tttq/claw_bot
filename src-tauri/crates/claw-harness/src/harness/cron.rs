// Claw Desktop - Cron定时任务 - 定时执行Agent任务
use claw_db::db::get_db;
use sea_orm::{ConnectionTrait, Statement};
use serde::{Deserialize, Serialize};

/// Cron定时任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: String,
    pub prompt: String,
    pub agent_id: Option<String>,
    pub delivery_channel_id: Option<String>,
    pub delivery_chat_id: Option<String>,
    pub enabled: bool,
    pub silent_on_empty: bool,
    pub last_run_at: Option<i64>,
    pub next_run_at: Option<i64>,
    pub run_count: i64,
    pub last_result: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Cron运行日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronRunLog {
    pub id: i64,
    pub cron_job_id: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub status: String,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

/// Cron任务存储 — 数据库CRUD操作
pub struct CronStore;

impl CronStore {
    /// 创建新的Cron任务
    pub async fn create(job: &CronJob) -> Result<(), String> {
        let db = get_db().await;
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO cron_jobs (id, name, schedule, prompt, agent_id, delivery_channel_id, delivery_chat_id, enabled, silent_on_empty, last_run_at, next_run_at, run_count, last_result, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            [
                job.id.clone().into(), job.name.clone().into(), job.schedule.clone().into(),
                job.prompt.clone().into(), job.agent_id.clone().into(), job.delivery_channel_id.clone().into(),
                job.delivery_chat_id.clone().into(), job.enabled.into(), job.silent_on_empty.into(),
                job.last_run_at.into(), job.next_run_at.into(), job.run_count.into(),
                job.last_result.clone().into(), job.created_at.into(), job.updated_at.into(),
            ],
        )).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 根据ID获取Cron任务
    pub async fn get(id: &str) -> Result<Option<CronJob>, String> {
        let db = get_db().await;
        let rows = db
            .query_all(Statement::from_sql_and_values(
                db.get_database_backend(),
                "SELECT * FROM cron_jobs WHERE id = ?1",
                [id.into()],
            ))
            .await
            .map_err(|e| e.to_string())?;

        rows.first().map(|row| row_to_cron_job(row)).transpose()
    }

    /// 列出所有Cron任务
    pub async fn list() -> Result<Vec<CronJob>, String> {
        let db = get_db().await;
        let rows = db
            .query_all(Statement::from_sql_and_values(
                db.get_database_backend(),
                "SELECT * FROM cron_jobs ORDER BY created_at DESC",
                [],
            ))
            .await
            .map_err(|e| e.to_string())?;

        rows.iter().map(|row| row_to_cron_job(row)).collect()
    }

    /// 更新Cron任务
    pub async fn update(job: &CronJob) -> Result<(), String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE cron_jobs SET name=?2, schedule=?3, prompt=?4, agent_id=?5, delivery_channel_id=?6, delivery_chat_id=?7, enabled=?8, silent_on_empty=?9, next_run_at=?10, updated_at=?11 WHERE id=?1",
            [
                job.id.clone().into(), job.name.clone().into(), job.schedule.clone().into(),
                job.prompt.clone().into(), job.agent_id.clone().into(), job.delivery_channel_id.clone().into(),
                job.delivery_chat_id.clone().into(), job.enabled.into(), job.silent_on_empty.into(),
                job.next_run_at.into(), now.into(),
            ],
        )).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 删除Cron任务
    pub async fn delete(id: &str) -> Result<(), String> {
        let db = get_db().await;
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM cron_jobs WHERE id = ?1",
            [id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 标记任务已运行 — 更新last_run_at、递增run_count、记录last_result
    pub async fn mark_run(id: &str, result: Option<&str>) -> Result<(), String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE cron_jobs SET last_run_at=?2, run_count=run_count+1, last_result=?3, updated_at=?2 WHERE id=?1",
            [id.into(), now.into(), result.into()],
        )).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 获取到期的Cron任务 — 返回所有enabled且next_run_at<=当前时间的任务
    pub async fn get_due_jobs() -> Result<Vec<CronJob>, String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        let rows = db.query_all(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT * FROM cron_jobs WHERE enabled = 1 AND (next_run_at IS NULL OR next_run_at <= ?1)",
            [now.into()],
        )).await.map_err(|e| e.to_string())?;

        rows.iter().map(|row| row_to_cron_job(row)).collect()
    }

    /// 记录运行日志 — 插入一条CronRunLog
    pub async fn log_run(log: &CronRunLog) -> Result<(), String> {
        let db = get_db().await;
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO cron_run_log (cron_job_id, started_at, finished_at, status, result_summary, error_message) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                log.cron_job_id.clone().into(), log.started_at.into(), log.finished_at.into(),
                log.status.clone().into(), log.result_summary.clone().into(), log.error_message.clone().into(),
            ],
        )).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// 数据库行转换为CronJob结构体
fn row_to_cron_job(row: &sea_orm::QueryResult) -> Result<CronJob, String> {
    Ok(CronJob {
        id: row.try_get::<String>("", "id").unwrap_or_default(),
        name: row.try_get::<String>("", "name").unwrap_or_default(),
        schedule: row.try_get::<String>("", "schedule").unwrap_or_default(),
        prompt: row.try_get::<String>("", "prompt").unwrap_or_default(),
        agent_id: row.try_get::<Option<String>>("", "agent_id").ok().flatten(),
        delivery_channel_id: row
            .try_get::<Option<String>>("", "delivery_channel_id")
            .ok()
            .flatten(),
        delivery_chat_id: row
            .try_get::<Option<String>>("", "delivery_chat_id")
            .ok()
            .flatten(),
        enabled: row.try_get::<bool>("", "enabled").unwrap_or(true),
        silent_on_empty: row.try_get::<bool>("", "silent_on_empty").unwrap_or(false),
        last_run_at: row.try_get::<Option<i64>>("", "last_run_at").ok().flatten(),
        next_run_at: row.try_get::<Option<i64>>("", "next_run_at").ok().flatten(),
        run_count: row.try_get::<i64>("", "run_count").unwrap_or(0),
        last_result: row
            .try_get::<Option<String>>("", "last_result")
            .ok()
            .flatten(),
        created_at: row.try_get::<i64>("", "created_at").unwrap_or(0),
        updated_at: row.try_get::<i64>("", "updated_at").unwrap_or(0),
    })
}

use serde::Serialize;
use sqlx::Row;
use tauri::Manager;

use crate::core;

pub mod schema;

pub use schema::sqlite_migrations;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportJobStateRecord {
	pub id: String,
	pub created_at: Option<String>,
	pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PauseResumeFlags {
	pub is_paused: bool,
	pub is_stopped: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessedZipStatusRecord {
	pub job_id: String,
	pub filename: String,
	pub status: Option<String>,
}

fn memories_db_url(app: &tauri::AppHandle) -> Result<String, String> {
	let mut app_data_dir = app
		.path()
		.app_data_dir()
		.map_err(|error| format!("failed to resolve app data directory: {error}"))?;

	std::fs::create_dir_all(&app_data_dir)
		.map_err(|error| format!("failed to create app data directory: {error}"))?;

	app_data_dir.push("memories.db");

	Ok(core::sqlite_url_from_path(&app_data_dir))
}

#[tauri::command]
pub async fn db_get_export_job_state(
	app: tauri::AppHandle,
) -> Result<Option<ExportJobStateRecord>, String> {
	let database_url = memories_db_url(&app)?;
	let pool = sqlx::SqlitePool::connect(&database_url)
		.await
		.map_err(|error| format!("failed to connect to memories database: {error}"))?;

	let row = sqlx::query(
		"SELECT id, created_at, status FROM ExportJobs ORDER BY created_at DESC, id DESC LIMIT 1",
	)
	.fetch_optional(&pool)
	.await
	.map_err(|error| format!("failed to read export job state from ExportJobs: {error}"))?;

	pool.close().await;

	Ok(row.map(|row| ExportJobStateRecord {
		id: row.get::<String, _>("id"),
		created_at: row.get::<Option<String>, _>("created_at"),
		status: row.get::<Option<String>, _>("status"),
	}))
}

#[tauri::command]
pub fn db_get_pause_resume_flags() -> Result<PauseResumeFlags, String> {
	let state = core::state::snapshot();

	Ok(PauseResumeFlags {
		is_paused: state.is_paused,
		is_stopped: state.is_stopped,
	})
}

#[tauri::command]
pub async fn db_get_zip_status(
	app: tauri::AppHandle,
	job_id: Option<String>,
) -> Result<Vec<ProcessedZipStatusRecord>, String> {
	let database_url = memories_db_url(&app)?;
	let pool = sqlx::SqlitePool::connect(&database_url)
		.await
		.map_err(|error| format!("failed to connect to memories database: {error}"))?;

	let rows = if let Some(job_id) = job_id {
		sqlx::query(
			"
			SELECT job_id, filename, status
			FROM ProcessedZips
			WHERE job_id = ?1
			ORDER BY filename ASC
			",
		)
		.bind(job_id)
		.fetch_all(&pool)
		.await
		.map_err(|error| format!("failed to read ProcessedZips for job: {error}"))?
	} else {
		sqlx::query(
			"
			SELECT job_id, filename, status
			FROM ProcessedZips
			ORDER BY job_id ASC, filename ASC
			",
		)
		.fetch_all(&pool)
		.await
		.map_err(|error| format!("failed to read ProcessedZips status: {error}"))?
	};

	pool.close().await;

	Ok(rows
		.into_iter()
		.map(|row| ProcessedZipStatusRecord {
			job_id: row.get::<String, _>("job_id"),
			filename: row.get::<String, _>("filename"),
			status: row.get::<Option<String>, _>("status"),
		})
		.collect())
}

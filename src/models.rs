use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingRequest {
  pub value: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {
  pub value: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScheduleTime {
  DateTime(String), // ISO 8601 datetime string
  Duration(u64),    // Duration in seconds from now
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleTaskRequest {
  pub task_name: String,
  pub schedule_time: ScheduleTime,
  pub parameters: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleTaskResponse {
  pub task_id: String,
  pub success: bool,
  pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskRequest {
  pub task_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelTaskResponse {
  pub success: bool,
  pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksResponse {
  pub tasks: Vec<TaskInfo>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskInfo {
  pub task_id: String,
  pub task_name: String,
  pub scheduled_time: String,
  pub status: TaskStatus,
  pub parameters: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
  Scheduled,
  Running,
  Completed,
  Failed,
  Cancelled,
}

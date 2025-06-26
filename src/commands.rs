use tauri::{AppHandle, command, Runtime};

use crate::models::*;
use crate::Result;
use crate::ScheduleTaskExt;

#[command]
pub(crate) async fn ping<R: Runtime>(
    app: AppHandle<R>,
    payload: PingRequest,
) -> Result<PingResponse> {
    app.schedule_task().ping(payload)
}

#[command]
pub(crate) async fn schedule_task<R: Runtime>(
    app: AppHandle<R>,
    payload: ScheduleTaskRequest,
) -> Result<ScheduleTaskResponse> {
    app.schedule_task().schedule_task(payload)
}

#[command]
pub(crate) async fn cancel_task<R: Runtime>(
    app: AppHandle<R>,
    payload: CancelTaskRequest,
) -> Result<CancelTaskResponse> {
    app.schedule_task().cancel_task(payload)
}

#[command]
pub(crate) async fn list_tasks<R: Runtime>(
    app: AppHandle<R>,
) -> Result<ListTasksResponse> {
    app.schedule_task().list_tasks()
}

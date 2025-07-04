use tauri::{AppHandle, command, Runtime};

use crate::models::*;
use crate::Result;
use crate::ScheduleTaskExt;

#[command]
pub(crate) async fn schedule_task<R: Runtime>(
    app: AppHandle<R>,
    payload: ScheduleTaskRequest,
) -> Result<ScheduleTaskResponse> {
    let task_result = app.schedule_task().schedule_task(payload);
        
    match task_result.await {
        Ok(response) => Ok(response),
        Err(e) => Err(e),
    }
    //foda
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

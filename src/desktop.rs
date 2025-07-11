use serde::de::DeserializeOwned;
use tauri::Manager;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Duration, Local, Utc};
use tokio_cron_scheduler::{Job, JobScheduler, job::JobId};

use crate::models::*;
use crate::ScheduledTaskHandler;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
  handler: Option<Arc<dyn ScheduledTaskHandler + Send + Sync>>,
) -> crate::Result<ScheduleTask<R>> {


  // initialize the job scheduler  
  let scheduler = tauri::async_runtime::block_on(async { JobScheduler::new().await.unwrap() });
  let scheduler = Arc::new(scheduler);
  let s = scheduler.clone();
  tauri::async_runtime::spawn(async move {
  //std::thread::spawn(move || {
      tauri::async_runtime::block_on(async move {
          s.start().await.unwrap();
      });
  });
  app.manage(scheduler);

  Ok(ScheduleTask {
    app: app.clone(),
    scheduled_tasks: Arc::new(Mutex::new(HashMap::new())),
    job_ids: Arc::new(Mutex::new(HashMap::new())),
    handler,
  })
}

/// Access to the schedule-task APIs.
pub struct ScheduleTask<R: Runtime> {
  app: AppHandle<R>,
  scheduled_tasks: Arc<Mutex<HashMap<String, TaskInfo>>>,
  job_ids: Arc<Mutex<HashMap<String, JobId>>>,
  handler: Option<Arc<dyn ScheduledTaskHandler + Send + Sync>>,
}

impl<R: Runtime> ScheduleTask<R> {
  pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
    Ok(PingResponse {
      value: payload.value,
    })
  }

  pub async fn schedule_task(&self, payload: ScheduleTaskRequest) -> crate::Result<ScheduleTaskResponse> {
    dbg!("Scheduling task with todo: {:?}", &payload);
    let payload = payload.clone();
    let schedule_time = match payload.schedule_time {
      ScheduleTime::DateTime(_) => payload.schedule_time,
      ScheduleTime::Duration(seconds) => {
        let scheduled_time = Local::now() + Duration::seconds(seconds as i64);
        ScheduleTime::DateTime(scheduled_time.to_rfc3339())
      }
    };
    // now converts the schedule_time to a Duration
    let now = Local::now();
    let duration = match schedule_time.clone() {
      ScheduleTime::DateTime(dt_str) => {
        let dt = DateTime::<Utc>::from_str(&dt_str)
          .map_err(|e| crate::Error::Generic(format!("Invalid datetime format: {}", e)))?;
        let local_dt = dt.with_timezone(&Local);
        local_dt.signed_duration_since(now)
      },
      ScheduleTime::Duration(seconds) => Duration::seconds(seconds as i64),
    };
    // spawns a new task and waits for the duration to elapse
    let scheduled_tasks = self.scheduled_tasks.clone();
    let handler = self.handler.clone();
    let task_id = Uuid::new_v4().to_string();
    let task_info = TaskInfo {
      task_id: task_id.clone(),
      task_name: payload.task_name.clone(),
      scheduled_time: match &schedule_time {
        ScheduleTime::DateTime(dt_str) => dt_str.clone(),
        ScheduleTime::Duration(seconds) => (Local::now() + Duration::seconds(*seconds as i64)).to_rfc3339(),
      },
      status: TaskStatus::Scheduled,
      parameters: payload.parameters.clone(),
    };

    {
      let mut tasks = scheduled_tasks.lock().unwrap();
      tasks.insert(task_id.clone(), task_info);
    }

    tokio::spawn({
      let scheduled_tasks = scheduled_tasks.clone();
      let handler = handler.clone();
      let task_id = task_id.clone();
      let task_name = payload.task_name.clone();
      let parameters = payload.parameters.clone().unwrap_or_default();
      async move {
        tokio::time::sleep(duration.to_std().unwrap()).await;
        {
          let mut tasks = scheduled_tasks.lock().unwrap();
          if let Some(task) = tasks.get_mut(&task_id) {
            task.status = TaskStatus::Running;
          }
        }
        if let Some(handler) = handler.as_ref() {
          let handler = handler.clone();
          let scheduled_tasks = scheduled_tasks.clone();
          tokio::spawn(async move {
            let _ = handler.handle_scheduled_task(&task_name, parameters);
            let mut tasks = scheduled_tasks.lock().unwrap();
            if let Some(task) = tasks.get_mut(&task_id) {
              task.status = TaskStatus::Completed;
            }
          });
        }
      }
    });
    Ok(ScheduleTaskResponse {
      task_id,
      success: true,
      message: Some("Task scheduled successfully".to_string()),
    })
  }


  pub fn cancel_task(&self, payload: CancelTaskRequest) -> crate::Result<CancelTaskResponse> {
    let mut job_ids = self.job_ids.lock().unwrap();
    let result = if let Some(job_id) = job_ids.remove(&payload.task_id) {
      tauri::async_runtime::block_on(async {
        self.app.state::<Arc<JobScheduler>>()
          .remove(&job_id)
          .await
      })
    } else {
      Ok(())
    };

    {
      let mut tasks = self.scheduled_tasks.lock().unwrap();
      if let Some(task) = tasks.get_mut(&payload.task_id) {
        task.status = TaskStatus::Cancelled;
      }
    }

    match result {
      Ok(_) => Ok(CancelTaskResponse {
        success: true,
        message: Some("Task cancelled successfully".to_string()),
      }),
      Err(e) => Ok(CancelTaskResponse {
        success: false,
        message: Some(format!("Failed to cancel task: {}", e)),
      }),
    }
  }

  pub fn list_tasks(&self) -> crate::Result<ListTasksResponse> {
    let tasks = self.scheduled_tasks.lock().unwrap();
    let task_list: Vec<TaskInfo> = tasks.values().cloned().collect();
    Ok(ListTasksResponse {
      tasks: task_list,
    })
  }
}


use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Duration, Local, Utc};
use tokio_cron_scheduler::{Job, JobScheduler, job::JobId};
use once_cell::sync::Lazy;

use crate::models::*;

static SCHEDULER: Lazy<Arc<JobScheduler>> = Lazy::new(|| {
    let scheduler = tauri::async_runtime::block_on(async { JobScheduler::new().await.unwrap() });
    let scheduler = Arc::new(scheduler);
    let s = scheduler.clone();
    std::thread::spawn(move || {
        tauri::async_runtime::block_on(async move {
            s.start().await.unwrap();
        });
    });
    scheduler
});

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<ScheduleTask<R>> {
  Ok(ScheduleTask {
    _app: app.clone(),
    scheduled_tasks: Arc::new(Mutex::new(HashMap::new())),
    job_ids: Arc::new(Mutex::new(HashMap::new())),
  })
}

/// Access to the schedule-task APIs.
pub struct ScheduleTask<R: Runtime> {
  _app: AppHandle<R>,
  scheduled_tasks: Arc<Mutex<HashMap<String, TaskInfo>>>,
  job_ids: Arc<Mutex<HashMap<String, JobId>>>,
}

impl<R: Runtime> ScheduleTask<R> {
  pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
    Ok(PingResponse {
      value: payload.value,
    })
  }

  pub fn schedule_task(&self, payload: ScheduleTaskRequest) -> crate::Result<ScheduleTaskResponse> {
    let task_id = Uuid::new_v4().to_string();
    let scheduled_time = match &payload.schedule_time {
      ScheduleTime::DateTime(dt_str) => {
        DateTime::<Utc>::from_str(dt_str.as_str())
          .map_err(|e| crate::Error::Generic(format!("Invalid datetime format: {}", e)))?
          .with_timezone(&Local)
      },
      ScheduleTime::Duration(seconds) => {
        Local::now() + Duration::seconds(*seconds as i64)
      }
    };

    let task_info = TaskInfo {
      task_id: task_id.clone(),
      task_name: payload.task_name.clone(),
      scheduled_time: scheduled_time.to_rfc3339(),
      status: TaskStatus::Scheduled,
    };

    {
      let mut tasks = self.scheduled_tasks.lock().unwrap();
      tasks.insert(task_id.clone(), task_info);
    }

    // Build cron expression for one-shot execution
    use chrono::{Datelike, Timelike};
    let cron_expr = format!(
      "{} {}  {}  {}  {}",
      scheduled_time.minute(),
      scheduled_time.hour(),
      scheduled_time.day(),
      scheduled_time.month(),
      scheduled_time.year()
    );

    let job_id = JobId::new_v4();
    let scheduled_tasks = self.scheduled_tasks.clone();
    let job_task_id = task_id.clone();
    
    dbg!("THis is the cron line we'll use: {}", &cron_expr);
    let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
      let scheduled_tasks = scheduled_tasks.clone();
      let job_task_id = job_task_id.clone();
      Box::pin(async move {
        let mut tasks = scheduled_tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(&job_task_id) {
          task.status = TaskStatus::Running;
        }
        // Here you would call the handler for the task
        // For now, just mark as completed
        if let Some(task) = tasks.get_mut(&job_task_id) {
          task.status = TaskStatus::Completed;
        }
      })
    }).map_err(|e| crate::Error::Generic(format!("Failed to create job: {}", e)))?;

    let result = tauri::async_runtime::block_on(async {
      SCHEDULER.add(job).await
    });

    match result {
      Ok(_) => {
        let mut job_ids = self.job_ids.lock().unwrap();
        job_ids.insert(task_id.clone(), job_id);
        Ok(ScheduleTaskResponse {
          task_id,
          success: true,
          message: Some("Task scheduled successfully".to_string()),
        })
      },
      Err(e) => {
        let mut tasks = self.scheduled_tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(&task_id) {
          task.status = TaskStatus::Failed;
        }
        Ok(ScheduleTaskResponse {
          task_id,
          success: false,
          message: Some(format!("Failed to schedule task: {}", e)),
        })
      }
    }
  }

  pub fn cancel_task(&self, payload: CancelTaskRequest) -> crate::Result<CancelTaskResponse> {
    let mut job_ids = self.job_ids.lock().unwrap();
    let result = if let Some(job_id) = job_ids.remove(&payload.task_id) {
      tauri::async_runtime::block_on(async {
        SCHEDULER.remove(&job_id).await
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


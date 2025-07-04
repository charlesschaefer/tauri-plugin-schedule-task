use serde::de::DeserializeOwned;
use tauri::async_runtime::JoinHandle;
use tauri::Manager;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use tokio_cron_scheduler::JobSchedulerError;
use std::collections::HashMap;
use std::future::IntoFuture;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use chrono::{DateTime, Duration, Local, Utc};
use tokio_cron_scheduler::{Job, JobScheduler, job::JobId, job::job_data::ListOfJobsAndNotifications};
use once_cell::sync::Lazy;
use crate::actor::*;

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

  pub async  fn schedule_task(&self, payload: ScheduleTaskRequest) -> crate::Result<ScheduleTaskResponse> {
    let task_id = Uuid::new_v4().to_string();
    let scheduled_time = match &payload.schedule_time {
      ScheduleTime::DateTime(dt_str) => {
        DateTime::<Utc>::from_str(dt_str.as_str())
          .map_err(|e| crate::Error::Generic(format!("Invalid datetime format: {}", e))).unwrap()
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
      parameters: payload.parameters.clone(),
    };

    {
      let mut tasks = self.scheduled_tasks.lock().unwrap();
      tasks.insert(task_id.clone(), task_info);
    }

    // Build cron expression for one-shot execution
    use chrono::{Datelike, Timelike};

    // sec   min   hour   day of month   month   day of week
    // *     *     *      *              *       *
    let cron_expr = format!(
      "0  {}  {}  {}  {}  *",
      scheduled_time.minute(),
      scheduled_time.hour(),
      scheduled_time.day(),
      scheduled_time.month()
    );

    let job_id = JobId::new_v4();
    let job_task_id = task_id.clone();
    let handler = self.handler.clone();

    dbg!("THis is the cron line we'll use: {}", &cron_expr);
    
    let job_ids = self.job_ids.clone();
    //let scheduled_tasks = self.scheduled_tasks.clone();
    //let job = Job::new_async_tz(cron_expr.as_str(), Utc, move |_uuid, _l| {
    let scheduled_tasks = self.scheduled_tasks.lock().unwrap();
    let job = Job::new_async_tz(scheduled_time.to_rfc3339().as_str(), Utc, move |_uuid, _l| {
      let job_task_id = job_task_id.clone();
      //let scheduled_tasks = scheduled_tasks.clone();
      let handler = handler.clone();
      let mut scheduled_tasks = scheduled_tasks.clone();
      Box::pin(async move {
        let tasks = scheduled_tasks.clone();
        let handler = handler.clone();
        dbg!("Running scheduled task with ID: {}", _uuid);
        if let Some(task) = tasks.get_mut(&job_task_id) {
          task.status = TaskStatus::Running;

          if let Some(handler) = handler.as_ref() {
            
            let task_name = task.task_name.clone();
            let parameters = task.parameters.clone().unwrap_or_default();
            let handler = handler.clone();
            tokio::spawn(async move  {
              let _ = handler.handle_scheduled_task(
                &task_name,
                parameters,
              );
              task.status = TaskStatus::Completed;
            });
          }
        }
      })
    }).map_err(|e| crate::Error::Generic(format!("Failed to create job: {}", e))).unwrap();
    
    let result = self.app.state::<Arc<JobScheduler>>().add(job).await;
    
    match result {
      Ok(_) => {
        let mut job_ids = job_ids.lock().unwrap();
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


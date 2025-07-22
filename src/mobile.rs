use std::{collections::HashMap, sync::{Arc, Mutex}};

use serde::{de::DeserializeOwned};
use tauri::{
  ipc::{Channel, InvokeResponseBody}, plugin::{PluginApi, PluginHandle}, AppHandle, Manager, Runtime
};
use tokio_cron_scheduler::job::JobId;

use crate::{models::*, ScheduleTaskExt, ScheduledTaskHandler};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_schedule_task);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  api: PluginApi<R, C>,
  handler: Option<Arc<dyn ScheduledTaskHandler<R> + Send + Sync>>,
) -> crate::Result<ScheduleTask<R>> {
  #[cfg(target_os = "android")]
  let handle = api.register_android_plugin("com.plugin.scheduletask", "ScheduleTaskPlugin")?;
  #[cfg(target_os = "ios")]
  let handle = api.register_ios_plugin(init_plugin_schedule_task)?;
  
  Ok(ScheduleTask {
    handle,
    app: app.clone(),
    scheduled_tasks: Arc::new(Mutex::new(HashMap::new())),
    job_ids: Arc::new(Mutex::new(HashMap::new())),
    task_handler: handler,
  })
}

/// Access to the schedule-task APIs.
pub struct ScheduleTask<R: Runtime> {
  app: AppHandle<R>,
  scheduled_tasks: Arc<Mutex<HashMap<String, TaskInfo>>>,
  job_ids: Arc<Mutex<HashMap<String, JobId>>>,
  handle: PluginHandle<R>,
  task_handler: Option<Arc<dyn ScheduledTaskHandler<R> + Send + Sync>>,
}

impl<R: Runtime> ScheduleTask<R> {
  pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
    self
      .handle
      .run_mobile_plugin("ping", payload)
      .map_err(Into::into)
  }

  pub async fn schedule_task(&self, payload: ScheduleTaskRequest) -> crate::Result<ScheduleTaskResponse> {
    self
      .handle
      .run_mobile_plugin("scheduleTask", payload)
      .map_err(Into::into)
  }

  pub fn cancel_task(&self, payload: CancelTaskRequest) -> crate::Result<CancelTaskResponse> {
    self
      .handle
      .run_mobile_plugin("cancelTask", payload)
      .map_err(Into::into)
  }

  pub fn list_tasks(&self) -> crate::Result<ListTasksResponse> {
    self
      .handle
      .run_mobile_plugin("listTasks", ())
      .map_err(Into::into)
  }

  pub fn set_task_handler(&self) -> crate::Result<()> {
    let app = self.app.clone();
    self
      .handle
      .run_mobile_plugin::<()>(
        "setEventHandler", 
        EventHandler {
          handle: Channel::new(move |event| {
            let event_data = match event {
                InvokeResponseBody::Json(payload) => {
                    serde_json::from_str::<TaskInfo>(&payload)
                        .ok()
                        //.map(|payload| payload.url)
                        //.unwrap()
                }
                _ => None,
            };

            dbg!("Receiving the schedule task with this data: {:?}", &event_data);

            let state = app.state::<ScheduleTask<R>>();
            if let Some(event_data) = event_data {
              if let Some(handler) = &state.task_handler {
                dbg!("Trying to run the event handler for task: {} with parameters {}", &event_data.task_name, &event_data.parameters);
                handler.handle_scheduled_task(&event_data.task_name, event_data.parameters.unwrap(), &app).unwrap();
              }
            }
            Ok(())
          })
        })
      .map_err(Into::into)
  }
}


#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventHandler {
  pub handle: Channel,
}

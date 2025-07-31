use std::{collections::HashMap, sync::{Arc, Mutex}};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tauri::{
  ipc::{Channel, InvokeResponseBody}, plugin::{PluginApi, PluginHandle}, AppHandle, Event, Manager, Runtime
};
use tokio_cron_scheduler::job::JobId;

use crate::{models::*, ScheduleTaskExt, ScheduledTaskHandler};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_schedule_task);

#[derive(Default)]
struct EventHandlerSet(bool);

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

  app.manage(EventHandlerSet(false));
  
  Ok(ScheduleTask {
    handle,
    app: app.clone(),
    scheduled_tasks: Arc::new(Mutex::new(HashMap::new())),
    job_ids: Arc::new(Mutex::new(HashMap::new())),
    task_handler: handler
  })
}

#[derive(Clone)]
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
    self.set_task_handler()?;
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
    let task_handler_set = self.app.state::<EventHandlerSet>();
    if task_handler_set.0 {
      return Ok(());
    }

    let app = self.app.clone();
    self.app.manage(EventHandlerSet(true));
    self
      .handle
      .run_mobile_plugin::<()>(
        "setEventHandler", 
        EventHandler {
          handler: Channel::new(move |event| {
            let event_data = match event {
                InvokeResponseBody::Json(payload) => {
                  dbg!("Received event data: {}", &payload);
                  serde_json::from_str::<serde_json::Value>(&payload)
                    .ok()
                    //.map(|payload| payload.url)
                    //.unwrap()
                }
                _ => None,
            };

            dbg!("Receiving the schedule task with this data: {:?}", &event_data);
            let (task_name, task_id, parameters) = match event_data {
              Some(data) => {
                let task_name = data.get("task_name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let task_id = data.get("task_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                let parameters = data.get("parameters")
                  .and_then(|v| v.as_object())
                  .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string())).collect())
                  .unwrap_or_default();
                (task_name, task_id, parameters)
              },
              None => (String::new(), String::new(), HashMap::new()),
            };

            let state = app.state::<ScheduleTask<R>>();
            //if let Some(event_data) = event_data {
              if let Some(handler) = &state.task_handler {
                dbg!("Trying to run the event handler for task: {} with parameters {}", &task_name, &parameters);
                handler.handle_scheduled_task(task_name.as_str(), parameters, &app).unwrap();
              }
            // }
            Ok(())
          })
        })
      .map_err(Into::into)
  }
}


#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventHandler {
  pub handler: Channel,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventInfo {
  pub task_id: String,
  pub task_name: String,
  pub parameters: HashMap<String, String>,
}
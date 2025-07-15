use serde::de::DeserializeOwned;
use tauri::{
  plugin::{PluginApi, PluginHandle},
  AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_schedule_task);

// initializes the Kotlin or Swift plugin classes
pub fn init<R: Runtime, C: DeserializeOwned>(
  _app: &AppHandle<R>,
  api: PluginApi<R, C>,
) -> crate::Result<ScheduleTask<R>> {
  #[cfg(target_os = "android")]
  let handle = api.register_android_plugin("com.plugin.scheduletask", "ScheduleTaskPlugin")?;
  #[cfg(target_os = "ios")]
  let handle = api.register_ios_plugin(init_plugin_schedule_task)?;
  Ok(ScheduleTask(handle))
}

/// Access to the schedule-task APIs.
pub struct ScheduleTask<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> ScheduleTask<R> {
  pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
    self
      .0
      .run_mobile_plugin("ping", payload)
      .map_err(Into::into)
  }

  pub async fn schedule_task(&self, payload: ScheduleTaskRequest) -> crate::Result<ScheduleTaskResponse> {
    self
      .0
      .run_mobile_plugin("scheduleTask", payload)
      .map_err(Into::into)
  }

  pub fn cancel_task(&self, payload: CancelTaskRequest) -> crate::Result<CancelTaskResponse> {
    self
      .0
      .run_mobile_plugin("cancelTask", payload)
      .map_err(Into::into)
  }

  pub fn list_tasks(&self) -> crate::Result<ListTasksResponse> {
    self
      .0
      .run_mobile_plugin("listTasks", ())
      .map_err(Into::into)
  }
}

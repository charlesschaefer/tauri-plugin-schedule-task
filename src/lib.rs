use tauri::{
  plugin::{Builder, TauriPlugin}, AppHandle, Manager, Runtime
};
use std::collections::HashMap;
use std::sync::Arc;

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::ScheduleTask;
#[cfg(mobile)]
use mobile::ScheduleTask;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the schedule-task APIs.
pub trait ScheduleTaskExt<R: Runtime> {
  fn schedule_task(&self) -> &ScheduleTask<R>;
}

impl<R: Runtime, T: Manager<R>> crate::ScheduleTaskExt<R> for T {
  fn schedule_task(&self) -> &ScheduleTask<R> {
    self.state::<ScheduleTask<R>>().inner()
  }
}

/// Trait for handling scheduled task execution
pub trait ScheduledTaskHandler<R: Runtime> {
  fn handle_scheduled_task(&self, task_name: &str, parameters: HashMap<String, String>, app: &AppHandle<R>) -> Result<()>;
}

/// Check if app was launched to run a scheduled task
pub fn check_scheduled_task_args() -> Option<(String, HashMap<String, String>)> {
  let args: Vec<String> = std::env::args().collect();
  
  let mut task_name: Option<String> = None;
  let mut parameters = HashMap::new();
  
  for arg in &args {
    if let Some(name) = arg.strip_prefix("--run-task=") {
      task_name = Some(name.to_string());
    } else if let Some(param) = arg.strip_prefix("--task-param=") {
      if let Some((key, value)) = param.split_once('=') {
        parameters.insert(key.to_string(), value.to_string());
      }
    }
  }
  
  task_name.map(|name| (name, parameters))
}

/// Initialize the plugin with a task handler
pub fn init_with_handler<R: Runtime, H: ScheduledTaskHandler<R> + Send + Sync + 'static>(
  handler: H,
) -> TauriPlugin<R> {
  let handler_arc = Arc::new(handler);
  Builder::new("schedule-task")
    .invoke_handler(tauri::generate_handler![
      commands::schedule_task,
      commands::cancel_task,
      commands::list_tasks
    ])
    .setup(move |app, api| {
      #[cfg(mobile)]
      let schedule_task = mobile::init(app, api, Some(handler_arc.clone()))?;
      #[cfg(desktop)]
      let schedule_task = desktop::init(app, api, Some(handler_arc.clone()))?;
      app.manage(schedule_task);
      
      // Check if this is a scheduled task execution
      #[cfg(desktop)]
      if let Some((task_name, parameters)) = check_scheduled_task_args() {
        let _ = handler_arc.handle_scheduled_task(&task_name, parameters, app);
        std::process::exit(0);
      }
      Ok(())
    })
    .build()
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
  Builder::new("schedule-task")
    .invoke_handler(tauri::generate_handler![
      commands::schedule_task,
      commands::cancel_task,
      commands::list_tasks
    ])
    .setup(|app, api| {
      #[cfg(mobile)]
      let schedule_task = mobile::init(app, api, None)?;
      #[cfg(desktop)]
      let schedule_task = desktop::init(app, api, None)?;
      app.manage(schedule_task);
      Ok(())
    })
    .build()
}

use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::process::Command;
use uuid::Uuid;
use chrono::{DateTime, Duration, Local, Utc};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
  app: &AppHandle<R>,
  _api: PluginApi<R, C>,
) -> crate::Result<ScheduleTask<R>> {
  Ok(ScheduleTask {
    _app: app.clone(),
    scheduled_tasks: Arc::new(Mutex::new(HashMap::new())),
  })
}

/// Access to the schedule-task APIs.
pub struct ScheduleTask<R: Runtime> {
  _app: AppHandle<R>,
  scheduled_tasks: Arc<Mutex<HashMap<String, TaskInfo>>>,
}

impl<R: Runtime> ScheduleTask<R> {
  pub fn ping(&self, payload: PingRequest) -> crate::Result<PingResponse> {
    Ok(PingResponse {
      value: payload.value,
    })
  }

  pub fn schedule_task(&self, payload: ScheduleTaskRequest) -> crate::Result<ScheduleTaskResponse> {
    let task_id = Uuid::new_v4().to_string();
    let app_exe = self.get_app_executable_path()?;
    
    let scheduled_time = match &payload.schedule_time {
      ScheduleTime::DateTime(dt_str) => {
        DateTime::<Utc>::from_str(dt_str.as_str()) //DateTime::parse_from_str(dt_str.as_str(),  "%Y-%m-%dT%H:%M:%S%.fZ")
          .map_err(|e| crate::Error::Generic(format!("Invalid datetime format: {}", e)))?
          //.with_timezone(&Utc)
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

    let result = self.schedule_with_os(&task_id, &payload.task_name, &scheduled_time, &app_exe, &payload.parameters);
    
    match result {
      Ok(_) => Ok(ScheduleTaskResponse {
        task_id,
        success: true,
        message: Some("Task scheduled successfully".to_string()),
      }),
      Err(e) => {
        {
          let mut tasks = self.scheduled_tasks.lock().unwrap();
          if let Some(task) = tasks.get_mut(&task_id) {
            task.status = TaskStatus::Failed;
          }
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
    let result = self.cancel_with_os(&payload.task_id);
    
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

  fn get_app_executable_path(&self) -> crate::Result<String> {
    std::env::current_exe()
      .map_err(|e| crate::Error::Generic(format!("Failed to get executable path: {}", e)))?
      .to_string_lossy()
      .to_string()
      .pipe(Ok)
  }

  #[cfg(target_os = "windows")]
  fn schedule_with_os(&self, task_id: &str, task_name: &str, scheduled_time: &DateTime<Local>, app_exe: &str, parameters: &Option<HashMap<String, String>>) -> crate::Result<()> {
    let task_args = format!("--run-task={}", task_name);
    let param_args = if let Some(params) = parameters {
      params.iter()
        .map(|(k, v)| format!("--task-param={}={}", k, v))
        .collect::<Vec<String>>()
        .join(" ")
    } else {
      String::new()
    };

    let full_args = if param_args.is_empty() {
      task_args
    } else {
      format!("{} {}", task_args, param_args)
    };

    let time_str = scheduled_time.format("%H:%M").to_string();
    let date_str = scheduled_time.format("%d/%m/%Y").to_string();

    let mut command = Command::new("schtasks");
    command.args(&[
        "/create",
        "/tn", &format!("TauriScheduledTask_{}", task_id),
        "/tr", &format!("\"{}\" {}", app_exe, full_args),
        "/sc", "once",
        "/st", &time_str,
        "/sd", &date_str,
        "/f"
      ]);
    let output = command
      .output()
      .map_err(|e| crate::Error::Generic(format!("Failed to execute schtasks: {}", e)))?;
    dbg!("Output: {:?}, command", &output, &command);

    if !output.status.success() {
      return Err(crate::Error::Generic(format!("schtasks failed: {}", String::from_utf8_lossy(&output.stderr))));
    }

    Ok(())
  }

  #[cfg(target_os = "linux")]
  fn schedule_with_os(&self, task_id: &str, task_name: &str, scheduled_time: &DateTime<Local>, app_exe: &str, parameters: &Option<HashMap<String, String>>) -> crate::Result<()> {
    use chrono::{Datelike, Timelike};

    let task_args = format!("--run-task={}", task_name);
    let param_args = if let Some(params) = parameters {
      params.iter()
        .map(|(k, v)| format!("--task-param={}={}", k, v))
        .collect::<Vec<String>>()
        .join(" ")
    } else {
      String::new()
    };

    let full_args = if param_args.is_empty() {
      task_args
    } else {
      format!("{} {}", task_args, param_args)
    };

    let cron_time = format!("{} {} {} {} *", 
      scheduled_time.minute(),
      scheduled_time.hour(),
      scheduled_time.day(),
      scheduled_time.month(),
    );

    let cron_entry = format!("{} \"{}\" {} >> /tmp/log_schedule 2>&1\n", cron_time, app_exe, full_args);
    
    let temp_file = format!("/tmp/tauri_cron_{}", task_id);
    std::fs::write(&temp_file, cron_entry)
      .map_err(|e| crate::Error::Generic(format!("Failed to write cron file: {}", e)))?;

    let output = Command::new("crontab")
      .arg(&temp_file)
      .output()
      .map_err(|e| crate::Error::Generic(format!("Failed to execute crontab: {}", e)))?;

    std::fs::remove_file(&temp_file)
      .map_err(|e| crate::Error::Generic(format!("Failed to remove temp file: {}", e)))?;

    if !output.status.success() {
      return Err(crate::Error::Generic(format!("crontab failed: {}", String::from_utf8_lossy(&output.stderr))));
    }

    Ok(())
  }

  #[cfg(target_os = "macos")]
  fn schedule_with_os(&self, task_id: &str, task_name: &str, scheduled_time: &DateTime<Local>, app_exe: &str, parameters: &Option<HashMap<String, String>>) -> crate::Result<()> {
    let task_args = format!("--run-task={}", task_name);
    let param_args = if let Some(params) = parameters {
      params.iter()
        .map(|(k, v)| format!("--task-param={}={}", k, v))
        .collect::<Vec<String>>()
        .join(" ")
    } else {
      String::new()
    };

    let full_args = if param_args.is_empty() {
      task_args
    } else {
      format!("{} {}", task_args, param_args)
    };

    let plist_content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.tauri.scheduled-task.{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>{}</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Year</key>
        <integer>{}</integer>
        <key>Month</key>
        <integer>{}</integer>
        <key>Day</key>
        <integer>{}</integer>
        <key>Hour</key>
        <integer>{}</integer>
        <key>Minute</key>
        <integer>{}</integer>
    </dict>
</dict>
</plist>"#, 
      task_id, 
      app_exe, 
      full_args,
      scheduled_time.year(),
      scheduled_time.month(),
      scheduled_time.day(),
      scheduled_time.hour(),
      scheduled_time.minute()
    );

    let plist_path = format!("/tmp/com.tauri.scheduled-task.{}.plist", task_id);
    std::fs::write(&plist_path, plist_content)
      .map_err(|e| crate::Error::Generic(format!("Failed to write plist file: {}", e)))?;

    let output = Command::new("launchctl")
      .args(&["load", &plist_path])
      .output()
      .map_err(|e| crate::Error::Generic(format!("Failed to execute launchctl: {}", e)))?;

    if !output.status.success() {
      std::fs::remove_file(&plist_path).ok();
      return Err(crate::Error::Generic(format!("launchctl failed: {}", String::from_utf8_lossy(&output.stderr))));
    }

    Ok(())
  }

  #[cfg(target_os = "windows")]
  fn cancel_with_os(&self, task_id: &str) -> crate::Result<()> {
    let output = Command::new("schtasks")
      .args(&["/delete", "/tn", &format!("TauriScheduledTask_{}", task_id), "/f"])
      .output()
      .map_err(|e| crate::Error::Generic(format!("Failed to execute schtasks: {}", e)))?;

    if !output.status.success() {
      return Err(crate::Error::Generic(format!("schtasks delete failed: {}", String::from_utf8_lossy(&output.stderr))));
    }

    Ok(())
  }

  #[cfg(target_os = "linux")]
  fn cancel_with_os(&self, task_id: &str) -> crate::Result<()> {
    let output = Command::new("crontab")
      .args(&["-l"])
      .output()
      .map_err(|e| crate::Error::Generic(format!("Failed to list crontab: {}", e)))?;

    if !output.status.success() {
      return Ok(());
    }

    let current_crontab = String::from_utf8_lossy(&output.stdout);
    let filtered_crontab: Vec<&str> = current_crontab
      .lines()
      .filter(|line| !line.contains(&format!("--run-task={}", task_id)))
      .collect();

    let temp_file = format!("/tmp/tauri_cron_filtered_{}", task_id);
    std::fs::write(&temp_file, filtered_crontab.join("\n"))
      .map_err(|e| crate::Error::Generic(format!("Failed to write filtered cron file: {}", e)))?;

    let output = Command::new("crontab")
      .arg(&temp_file)
      .output()
      .map_err(|e| crate::Error::Generic(format!("Failed to update crontab: {}", e)))?;

    std::fs::remove_file(&temp_file).ok();

    if !output.status.success() {
      return Err(crate::Error::Generic(format!("crontab update failed: {}", String::from_utf8_lossy(&output.stderr))));
    }

    Ok(())
  }

  #[cfg(target_os = "macos")]
  fn cancel_with_os(&self, task_id: &str) -> crate::Result<()> {
    let plist_path = format!("/tmp/com.tauri.scheduled-task.{}.plist", task_id);
    
    let output = Command::new("launchctl")
      .args(&["unload", &plist_path])
      .output()
      .map_err(|e| crate::Error::Generic(format!("Failed to execute launchctl: {}", e)))?;

    std::fs::remove_file(&plist_path).ok();

    if !output.status.success() {
      return Err(crate::Error::Generic(format!("launchctl unload failed: {}", String::from_utf8_lossy(&output.stderr))));
    }

    Ok(())
  }
}

trait PipeExt<T> {
  fn pipe<U, F: FnOnce(T) -> U>(self, f: F) -> U;
}

impl<T> PipeExt<T> for T {
  fn pipe<U, F: FnOnce(T) -> U>(self, f: F) -> U {
    f(self)
  }
}

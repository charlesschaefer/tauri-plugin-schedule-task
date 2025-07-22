#[cfg(mobile)]
use tauri::AppHandle;
#[cfg(desktop)]
use tauri::AppHandle;
use tauri::Runtime;
use tauri_plugin_schedule_task;

use std::collections::HashMap;
use tauri_plugin_schedule_task::{Result, ScheduledTaskHandler};

struct MyTaskHandler;

impl <R: Runtime> ScheduledTaskHandler<R> for MyTaskHandler {
    fn handle_scheduled_task(
        &self,
        task_name: &str,
        parameters: HashMap<String, String>,
        app: &tauri::AppHandle<R>,
    ) -> Result<()> {
        println!("Executing scheduled task: {}", task_name);

        // Log parameters
        for (key, value) in &parameters {
            println!("Parameter {}: {}", key, value);
        }

        match task_name {
            "backup" => {
                println!("Running backup task...");
                self.perform_backup(&parameters, app)?;
            }
            "cleanup" => {
                println!("Running cleanup task...");
                self.perform_cleanup(&parameters)?;
            }
            "send_report" => {
                println!("Sending daily report...");
                self.send_daily_report(&parameters)?;
            }
            "database_maintenance" => {
                println!("Running database maintenance...");
                self.run_db_maintenance(&parameters)?;
            }
            _ => {
                println!("Unknown task: {}", task_name);
                return Err(tauri_plugin_schedule_task::Error::Generic(format!(
                    "Unknown task: {}",
                    task_name
                )));
            }
        }

        Ok(())
    }
}

impl MyTaskHandler {
    #[cfg(desktop)]
    // Implement your task functions
    fn perform_backup(&self, params: &HashMap<String, String>, app: &AppHandle<R>) -> Result<()> {
        let default = &String::from("/tmp/backup");
        let backup_path = params.get("path").unwrap_or(default);
        println!("Backing up to: {}", backup_path);
        dbg!("Sending the notification for backup task");
        send_notification(
            app,
            "Backup Task",
            &format!("Backing up to: {}", backup_path),
        );
        // Your backup logic here
        Ok(())
    }
    
    #[cfg(mobile)]
    // Implement your task functions
    fn perform_backup<R: Runtime>(&self, params: &HashMap<String, String>, app: &AppHandle<R>) -> Result<()> {
        let default = &String::from("/tmp/backup");
        let backup_path = params.get("path").unwrap_or(default);
        println!("[MOBILE] Backing up to: {}", backup_path);
        dbg!("[MOBILE] Sending the notification for backup task");
        send_notification(
            app,
            "Backup Task",
            &format!("Backing up to: {}", backup_path),
        );
        // Your backup logic here
        Ok(())
    }
    
    fn perform_cleanup(&self, params: &HashMap<String, String>) -> Result<()> {
        let max_age_days: u32 = params
            .get("max_age_days")
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        println!("Cleaning up files older than {} days", max_age_days);
        // Your cleanup logic here
        Ok(())
    }
    
    fn send_daily_report(&self, params: &HashMap<String, String>) -> Result<()> {
        let default = &String::from("admin@example.com");
        let email = params.get("email").unwrap_or(default);
        println!("Sending report to: {}", email);
        // Your reporting logic here
        Ok(())
    }
    
    fn run_db_maintenance(&self, _params: &HashMap<String, String>) -> Result<()> {
        println!("Running database maintenance...");
        // Your database maintenance logic here
        Ok(())
    }
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_schedule_task::init_with_handler(MyTaskHandler))
        //.invoke_handler(tauri::generate_handler![])
        .setup(|app| {
            // Register the task handler
            send_notification(
                app.handle(),
                "Tauri App",
                "Scheduled Task Handler Initialized",
            );
            println!("Scheduled Task Handler Initialized and notification sent.");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn send_notification<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    app.notification().builder()
        .title(title)
        .body(body)
        .show()
        .expect("Failed to send notification");
}
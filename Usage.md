# Tauri Plugin Schedule Task - Usage Guide

This guide shows you how to use the `tauri-plugin-schedule-task` to schedule Rust code execution at specific times or after durations across all supported platforms (Windows, Linux, macOS, Android, iOS).

## Table of Contents

1. [Installation](#installation)
2. [Platform-Specific Setup](#platform-specific-setup)
3. [Basic Usage](#basic-usage)
4. [Scheduling Tasks](#scheduling-tasks)
5. [Task Management](#task-management)
6. [Frontend Integration](#frontend-integration)
7. [Advanced Examples](#advanced-examples)

## Installation

Add the plugin to your `Cargo.toml`:

```toml
[dependencies]
tauri-plugin-schedule-task = { path = "path/to/plugin" }
```

## Platform-Specific Setup

### Android

Add the following permissions to your `android/app/src/main/AndroidManifest.xml`:

```xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <!-- Required for scheduling tasks -->
    <uses-permission android:name="android.permission.SCHEDULE_EXACT_ALARM" />
    <uses-permission android:name="android.permission.USE_EXACT_ALARM" />
    <uses-permission android:name="android.permission.WAKE_LOCK" />
    <uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED" />
    
    <!-- For WorkManager (Android 6.0+) -->
    <uses-permission android:name="android.permission.REQUEST_IGNORE_BATTERY_OPTIMIZATIONS" />

    <application>
        <!-- Register the broadcast receiver for alarm-based scheduling -->
        <receiver android:name="com.plugin.scheduletask.ScheduledTaskReceiver"
                  android:enabled="true"
                  android:exported="false" />
        
        <!-- Boot receiver to reschedule tasks after device restart -->
        <receiver android:name="com.plugin.scheduletask.BootReceiver"
                  android:enabled="true"
                  android:exported="false">
            <intent-filter android:priority="1000">
                <action android:name="android.intent.action.BOOT_COMPLETED" />
                <action android:name="android.intent.action.MY_PACKAGE_REPLACED" />
                <action android:name="android.intent.action.PACKAGE_REPLACED" />
                <data android:scheme="package" />
            </intent-filter>
        </receiver>
    </application>
</manifest>
```

Add to your `android/app/build.gradle`:

```gradle
dependencies {
    implementation 'androidx.work:work-runtime-ktx:2.8.1'
}
```

### iOS

Add background capabilities to your `ios/Runner/Info.plist`:

```xml
<key>UIBackgroundModes</key>
<array>
    <string>background-processing</string>
    <string>background-fetch</string>
</array>

<key>BGTaskSchedulerPermittedIdentifiers</key>
<array>
    <string>com.tauri.scheduled-task.*</string>
</array>
```

Request notification permissions in your iOS app delegate or main app file:

```swift
import UserNotifications

// Request notification permissions
UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { granted, error in
    if let error = error {
        print("Notification permission error: \(error)")
    }
}
```

### Windows

No additional setup required. The plugin uses Windows Task Scheduler (`schtasks`).

### Linux

Ensure `cron` is installed and running on the system:

```bash
sudo systemctl enable cron
sudo systemctl start cron
```

### macOS

No additional setup required. The plugin uses `launchctl`.

## Basic Usage

### 1. Create a Task Handler

First, implement the `ScheduledTaskHandler` trait to define how your scheduled tasks should be executed:

```rust
use std::collections::HashMap;
use tauri_plugin_schedule_task::{ScheduledTaskHandler, Result};

struct MyTaskHandler;

impl ScheduledTaskHandler for MyTaskHandler {
    fn handle_scheduled_task(&self, task_name: &str, parameters: HashMap<String, String>) -> Result<()> {
        println!("Executing scheduled task: {}", task_name);
        
        // Log parameters
        for (key, value) in &parameters {
            println!("Parameter {}: {}", key, value);
        }
        
        match task_name {
            "backup" => {
                println!("Running backup task...");
                perform_backup(&parameters)?;
            }
            "cleanup" => {
                println!("Running cleanup task...");
                perform_cleanup(&parameters)?;
            }
            "send_report" => {
                println!("Sending daily report...");
                send_daily_report(&parameters)?;
            }
            "database_maintenance" => {
                println!("Running database maintenance...");
                run_db_maintenance(&parameters)?;
            }
            _ => {
                println!("Unknown task: {}", task_name);
                return Err(tauri_plugin_schedule_task::Error::Generic(
                    format!("Unknown task: {}", task_name)
                ));
            }
        }
        
        Ok(())
    }
}

// Implement your task functions
fn perform_backup(params: &HashMap<String, String>) -> Result<()> {
    let backup_path = params.get("path").unwrap_or(&String::from("/tmp/backup"));
    println!("Backing up to: {}", backup_path);
    // Your backup logic here
    Ok(())
}

fn perform_cleanup(params: &HashMap<String, String>) -> Result<()> {
    let max_age_days: u32 = params.get("max_age_days")
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);
    println!("Cleaning up files older than {} days", max_age_days);
    // Your cleanup logic here
    Ok(())
}

fn send_daily_report(params: &HashMap<String, String>) -> Result<()> {
    let email = params.get("email").unwrap_or(&String::from("admin@example.com"));
    println!("Sending report to: {}", email);
    // Your reporting logic here
    Ok(())
}

fn run_db_maintenance(_params: &HashMap<String, String>) -> Result<()> {
    println!("Running database maintenance...");
    // Your database maintenance logic here
    Ok(())
}
```

### 2. Initialize the Plugin

Add the plugin to your Tauri app with your task handler:

```rust
use tauri_plugin_schedule_task;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_schedule_task::init_with_handler(MyTaskHandler))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Important**: the plugin must be the first one to be initialized, thus allowing the desktop scheduling routines to be done before the full app-initialization. If the app's executable is being called with parameters to run a scheduled task, it will run the task routine and, after that, exit the process (avoiding a second instance of the app to be opened).

## Scheduling Tasks

### Frontend Commands

Create TypeScript interfaces for type safety:

```typescript
// src/types/schedule.ts
export interface ScheduleTime {
  dateTime?: string; // ISO 8601 format
  duration?: number; // seconds
}

export interface ScheduleTaskRequest {
  taskName: string;
  scheduleTime: ScheduleTime;
  parameters?: Record<string, string>;
}

export interface ScheduleTaskResponse {
  taskId: string;
  success: boolean;
  message?: string;
}

export interface TaskInfo {
  taskId: string;
  taskName: string;
  scheduledTime: string;
  status: 'Scheduled' | 'Running' | 'Completed' | 'Failed' | 'Cancelled';
}
```

### Schedule by Absolute Time

```typescript
import { invoke } from '@tauri-apps/api/core';

// Schedule a backup task for tomorrow 2 AM
async function scheduleBackupTask() {
  const tomorrow = new Date();
  tomorrow.setDate(tomorrow.getDate() + 1);
  tomorrow.setHours(2, 0, 0, 0);

  const request: ScheduleTaskRequest = {
    taskName: 'backup',
    scheduleTime: {
      dateTime: tomorrow.toISOString()
    },
    parameters: {
      path: '/home/user/backup',
      compression: 'gzip'
    }
  };

  try {
    const response: ScheduleTaskResponse = await invoke('plugin:schedule-task|schedule_task', {
      payload: request
    });
    
    if (response.success) {
      console.log(`Task scheduled with ID: ${response.taskId}`);
    } else {
      console.error(`Failed to schedule task: ${response.message}`);
    }
  } catch (error) {
    console.error('Error scheduling task:', error);
  }
}
```

### Schedule by Duration

```typescript
// Schedule a cleanup task to run in 1 hour
async function scheduleCleanupTask() {
  const request: ScheduleTaskRequest = {
    taskName: 'cleanup',
    scheduleTime: {
      duration: 3600 // 1 hour in seconds
    },
    parameters: {
      max_age_days: '7',
      target_directory: '/tmp'
    }
  };

  try {
    const response: ScheduleTaskResponse = await invoke('plugin:schedule-task|schedule_task', {
      payload: request
    });
    
    console.log(response.success ? 
      `Cleanup scheduled for 1 hour from now: ${response.taskId}` : 
      `Failed: ${response.message}`
    );
  } catch (error) {
    console.error('Error scheduling cleanup:', error);
  }
}
```

## Task Management

### List All Scheduled Tasks

```typescript
async function listScheduledTasks() {
  try {
    const response = await invoke('plugin:schedule-task|list_tasks');
    const tasks: TaskInfo[] = response.tasks;
    
    console.log('Scheduled tasks:');
    tasks.forEach(task => {
      console.log(`- ${task.taskName} (${task.taskId}): ${task.status} at ${task.scheduledTime}`);
    });
    
    return tasks;
  } catch (error) {
    console.error('Error listing tasks:', error);
    return [];
  }
}
```

### Cancel a Scheduled Task

```typescript
async function cancelTask(taskId: string) {
  try {
    const response = await invoke('plugin:schedule-task|cancel_task', {
      payload: { taskId }
    });
    
    console.log(response.success ? 
      'Task cancelled successfully' : 
      `Failed to cancel task: ${response.message}`
    );
    
    return response.success;
  } catch (error) {
    console.error('Error cancelling task:', error);
    return false;
  }
}
```

## Frontend Integration

### React Hook Example

```typescript
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface UseScheduledTasksReturn {
  tasks: TaskInfo[];
  scheduleTask: (request: ScheduleTaskRequest) => Promise<boolean>;
  cancelTask: (taskId: string) => Promise<boolean>;
  refreshTasks: () => Promise<void>;
  loading: boolean;
}

export function useScheduledTasks(): UseScheduledTasksReturn {
  const [tasks, setTasks] = useState<TaskInfo[]>([]);
  const [loading, setLoading] = useState(false);

  const refreshTasks = async () => {
    setLoading(true);
    try {
      const response = await invoke('plugin:schedule-task|list_tasks');
      setTasks(response.tasks || []);
    } catch (error) {
      console.error('Failed to refresh tasks:', error);
    } finally {
      setLoading(false);
    }
  };

  const scheduleTask = async (request: ScheduleTaskRequest): Promise<boolean> => {
    try {
      const response: ScheduleTaskResponse = await invoke('plugin:schedule-task|schedule_task', {
        payload: request
      });
      
      if (response.success) {
        await refreshTasks();
        return true;
      }
      return false;
    } catch (error) {
      console.error('Failed to schedule task:', error);
      return false;
    }
  };

  const cancelTask = async (taskId: string): Promise<boolean> => {
    try {
      const response = await invoke('plugin:schedule-task|cancel_task', {
        payload: { taskId }
      });
      
      if (response.success) {
        await refreshTasks();
        return true;
      }
      return false;
    } catch (error) {
      console.error('Failed to cancel task:', error);
      return false;
    }
  };

  useEffect(() => {
    refreshTasks();
  }, []);

  return { tasks, scheduleTask, cancelTask, refreshTasks, loading };
}
```

### Vue 3 Composable Example

```typescript
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export function useScheduledTasks() {
  const tasks = ref<TaskInfo[]>([]);
  const loading = ref(false);

  const refreshTasks = async () => {
    loading.value = true;
    try {
      const response = await invoke('plugin:schedule-task|list_tasks');
      tasks.value = response.tasks || [];
    } catch (error) {
      console.error('Failed to refresh tasks:', error);
    } finally {
      loading.value = false;
    }
  };

  const scheduleTask = async (request: ScheduleTaskRequest): Promise<boolean> => {
    try {
      const response: ScheduleTaskResponse = await invoke('plugin:schedule-task|schedule_task', {
        payload: request
      });
      
      if (response.success) {
        await refreshTasks();
        return true;
      }
      return false;
    } catch (error) {
      console.error('Failed to schedule task:', error);
      return false;
    }
  };

  onMounted(() => {
    refreshTasks();
  });

  return {
    tasks: readonly(tasks),
    loading: readonly(loading),
    scheduleTask,
    refreshTasks
  };
}
```

## Advanced Examples

### Platform-Specific Task Handling

If you need platform-specific behavior, use Rust's conditional compilation:

```rust
impl ScheduledTaskHandler for MyTaskHandler {
    fn handle_scheduled_task(&self, task_name: &str, parameters: HashMap<String, String>) -> Result<()> {
        match task_name {
            "system_backup" => {
                #[cfg(target_os = "windows")]
                {
                    // Windows-specific backup using robocopy
                    use std::process::Command;
                    let source = parameters.get("source").unwrap_or(&String::from("C:\\"));
                    let dest = parameters.get("dest").unwrap_or(&String::from("D:\\Backup"));
                    
                    let output = Command::new("robocopy")
                        .args(&[source, dest, "/MIR", "/R:3", "/W:10"])
                        .output()?;
                    
                    if output.status.success() {
                        println!("Windows backup completed successfully");
                    }
                }
                
                #[cfg(target_os = "linux")]
                {
                    // Linux-specific backup using rsync
                    use std::process::Command;
                    let source = parameters.get("source").unwrap_or(&String::from("/home"));
                    let dest = parameters.get("dest").unwrap_or(&String::from("/backup"));
                    
                    let output = Command::new("rsync")
                        .args(&["-avz", "--delete", source, dest])
                        .output()?;
                    
                    if output.status.success() {
                        println!("Linux backup completed successfully");
                    }
                }
                
                #[cfg(target_os = "macos")]
                {
                    // macOS-specific backup using Time Machine or custom logic
                    use std::process::Command;
                    let output = Command::new("tmutil")
                        .args(&["startbackup", "--auto"])
                        .output()?;
                    
                    if output.status.success() {
                        println!("macOS backup started successfully");
                    }
                }
                
                #[cfg(target_os = "android")]
                {
                    // Android-specific backup logic
                    println!("Android backup not implemented in this example");
                }
                
                #[cfg(target_os = "ios")]
                {
                    // iOS-specific backup logic
                    println!("iOS backup not implemented in this example");
                }
            }
            
            "send_notification" => {
                // Cross-platform notification
                send_cross_platform_notification(&parameters)?;
            }
            
            _ => {
                return Err(tauri_plugin_schedule_task::Error::Generic(
                    format!("Unknown task: {}", task_name)
                ));
            }
        }
        
        Ok(())
    }
}

fn send_cross_platform_notification(params: &HashMap<String, String>) -> Result<()> {
    let title = params.get("title").unwrap_or(&String::from("Scheduled Task"));
    let message = params.get("message").unwrap_or(&String::from("Task completed"));
    
    #[cfg(target_os = "windows")]
    {
        // Windows notification using toast
        use std::process::Command;
        let _ = Command::new("powershell")
            .args(&[
                "-Command",
                &format!(
                    "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.MessageBox]::Show('{}', '{}')",
                    message, title
                )
            ])
            .output();
    }
    
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Unix-like systems using notify-send or osascript
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            let _ = Command::new("notify-send")
                .args(&[title, message])
                .output();
        }
        
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let _ = Command::new("osascript")
                .args(&[
                    "-e",
                    &format!("display notification \"{}\" with title \"{}\"", message, title)
                ])
                .output();
        }
    }
    
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        // Mobile platforms - notifications are handled by the native plugin
        println!("Mobile notification: {} - {}", title, message);
    }
    
    Ok(())
}
```

### Complex Scheduling Patterns

```typescript
// Schedule recurring daily backup at 2 AM
async function scheduleRecurringBackup() {
  const scheduleNextBackup = async () => {
    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    tomorrow.setHours(2, 0, 0, 0);
    
    await invoke('plugin:schedule-task|schedule_task', {
      payload: {
        taskName: 'recurring_backup',
        scheduleTime: { dateTime: tomorrow.toISOString() },
        parameters: {
          path: '/backup',
          schedule_next: 'true' // Flag to schedule next occurrence
        }
      }
    });
  };
  
  await scheduleNextBackup();
}

// In your task handler, handle the recurring logic:
// match task_name {
//     "recurring_backup" => {
//         perform_backup(&parameters)?;
//         
//         // If this was a recurring task, schedule the next one
//         if parameters.get("schedule_next") == Some(&String::from("true")) {
//             // Schedule next occurrence (you'd need to implement this)
//             schedule_next_backup()?;
//         }
//     }
// }
```

### Error Handling and Logging

```rust
use log::{info, error, warn};

impl ScheduledTaskHandler for MyTaskHandler {
    fn handle_scheduled_task(&self, task_name: &str, parameters: HashMap<String, String>) -> Result<()> {
        info!("Starting scheduled task: {} with {} parameters", task_name, parameters.len());
        
        let start_time = std::time::Instant::now();
        
        let result = match task_name {
            "backup" => self.handle_backup_task(&parameters),
            "cleanup" => self.handle_cleanup_task(&parameters),
            "maintenance" => self.handle_maintenance_task(&parameters),
            unknown => {
                error!("Unknown task requested: {}", unknown);
                Err(tauri_plugin_schedule_task::Error::Generic(
                    format!("Unknown task: {}", unknown)
                ))
            }
        };
        
        let duration = start_time.elapsed();
        
        match &result {
            Ok(_) => info!("Task '{}' completed successfully in {:?}", task_name, duration),
            Err(e) => error!("Task '{}' failed after {:?}: {}", task_name, duration, e),
        }
        
        result
    }
}

impl MyTaskHandler {
    fn handle_backup_task(&self, params: &HashMap<String, String>) -> Result<()> {
        let backup_path = params.get("path")
            .ok_or_else(|| tauri_plugin_schedule_task::Error::Generic("Missing 'path' parameter".to_string()))?;
        
        info!("Starting backup to: {}", backup_path);
        
        // Your backup implementation
        std::fs::create_dir_all(backup_path)
            .map_err(|e| tauri_plugin_schedule_task::Error::Generic(format!("Failed to create backup directory: {}", e)))?;
        
        // Simulate backup work
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        info!("Backup completed successfully");
        Ok(())
    }
    
    fn handle_cleanup_task(&self, params: &HashMap<String, String>) -> Result<()> {
        let max_age_days: u32 = params.get("max_age_days")
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        
        let target_dir = params.get("target_directory").unwrap_or(&String::from("/tmp"));
        
        info!("Cleaning files older than {} days in {}", max_age_days, target_dir);
        
        // Your cleanup implementation here
        
        Ok(())
    }
    
    fn handle_maintenance_task(&self, _params: &HashMap<String, String>) -> Result<()> {
        info!("Running system maintenance");
        
        // Your maintenance implementation here
        
        Ok(())
    }
}
```

## Snap and Flatpak bundled Apps

### Flatpak
To be able to schedule a task from your flatpak bundled app, you'll need to add some permissions to the manifest file:
```yaml
app-id: com.example.my_app
runtime: org.gnome.Platform
runtime-version: '42'
sdk: org.gnome.Sdk
command: my_app
finish-args:
  - --socket=x11
  - --socket=pulseaudio
  - --share=network
  - --share=ipc
  - --filesystem=home
  ## Filesystem needed permissions
  - --filesystem=xdg-run/dconf:ro
  - --filesystem=xdg-config/dconf:ro
  - --filesystem=xdg-cache # Allow access to the cache directory
  - --filesystem=xdg-data  # Allow access to the data directory
  - --filesystem=~/.config/gconf:ro
  - --env=XDG_RUNTIME_DIR=/run/user/$UID # Required for running cron jobs
  - --env=XDG_DATA_HOME=/app/data # Required for running cron jobs
  ## DBus needed permissions
  - --own-name=org.gnome.SettingsDaemon.Clock # Allow access to the clock
  - --mount=type=bind,src=/path/to/host/directory,dest=/app/host/directory, [Optional]
modules:
  - name: my_app
    buildsystem: meson
    sources:
      - type: dir
        path: .
```

### Snap

To be able to schedule a task from your snap bundled app, you'll need to connect the app to the `cron` plug:

```yaml
   name: my-snap-name
   version: '1.0'
   # .... other fields
   parts:
     my-part:
       plugin: python3
       source: .
       # ... your part definition ...

   apps:
     my-app:
       command: bin/my-app
       plugs: [cron] # this plug is needed to use the OS's Crontab
```

## Security Considerations

1. **Validate Parameters**: Always validate and sanitize parameters passed to scheduled tasks
2. **Limit Task Names**: Use an allowlist of valid task names
3. **Resource Limits**: Implement timeouts and resource limits for long-running tasks
4. **Logging**: Log all scheduled task executions for audit purposes
5. **Permissions**: Request only necessary permissions on mobile platforms

## Troubleshooting

### Common Issues

1. **Tasks not executing on Android**: Check that the app has permission to run in the background and isn't being killed by battery optimization
2. **iOS tasks not running**: Ensure background app refresh is enabled and the app has notification permissions
3. **Linux cron issues**: Verify that the cron service is running and the user has permission to modify crontab
4. **Windows Task Scheduler**: Run the app with administrator privileges if tasks fail to schedule

### Debug Mode

Enable debug logging in your task handler:

```rust
#[cfg(debug_assertions)]
println!("DEBUG: Task '{}' called with params: {:?}", task_name, parameters);
```


This comprehensive guide should help you implement scheduled task execution across all platforms using the Tauri plugin.
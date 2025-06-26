import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterOutlet } from '@angular/router';
import { FormsModule } from "@angular/forms";
import { invoke } from "@tauri-apps/api/core";
import { DateTime } from "luxon";

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



@Component({
  selector: 'app-root',
  standalone: true,
  imports: [CommonModule, RouterOutlet, CommonModule, FormsModule],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent {
  greetingMessage = "";
  backupTime = "";
  currentTime = new Date();

  greet(event: SubmitEvent, name: string): void {
    event.preventDefault();

    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    invoke<string>("greet", { name }).then((text) => {
      this.greetingMessage = text;
    });
  }

  // Schedule a backup task for tomorrow 2 AM
  async scheduleBackupTask() {
    
    let backupTime = DateTime.fromJSDate(new Date(this.backupTime));

    const request: ScheduleTaskRequest = {
      taskName: 'backup',
      scheduleTime: {
        dateTime: backupTime.toISO() || undefined
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
}

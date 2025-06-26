import SwiftRs
import Tauri
import UIKit
import WebKit
import UserNotifications
import BackgroundTasks

class PingArgs: Decodable {
  let value: String?
}

class ScheduleTimeArgs: Decodable {
  let dateTime: String?
  let duration: Int?
}

class ScheduleTaskArgs: Decodable {
  let taskName: String
  let scheduleTime: ScheduleTimeArgs
  let parameters: [String: String]?
}

class CancelTaskArgs: Decodable {
  let taskId: String
}

struct ScheduledTaskInfo {
  let taskId: String
  let taskName: String
  let scheduledTime: String
  let status: String
}

class ExamplePlugin: Plugin {
  private var scheduledTasks: [String: ScheduledTaskInfo] = [:]
  private let notificationCenter = UNUserNotificationCenter.current()
  
  override func load(webview: WKWebView) {
    super.load(webview: webview)
    requestNotificationPermission()
  }
  
  @objc public func ping(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(PingArgs.self)
    invoke.resolve(["value": args.value ?? ""])
  }
  
  @objc public func scheduleTask(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(ScheduleTaskArgs.self)
    let taskId = UUID().uuidString
    
    do {
      let triggerDate: Date
      
      if let dateTimeStr = args.scheduleTime.dateTime {
        let formatter = ISO8601DateFormatter()
        guard let date = formatter.date(from: dateTimeStr) else {
          invoke.reject("Invalid datetime format")
          return
        }
        triggerDate = date
      } else if let duration = args.scheduleTime.duration {
        triggerDate = Date().addingTimeInterval(TimeInterval(duration))
      } else {
        invoke.reject("Invalid schedule time")
        return
      }
      
      try scheduleNotification(taskId: taskId, taskName: args.taskName, triggerDate: triggerDate, parameters: args.parameters)
      
      let taskInfo = ScheduledTaskInfo(
        taskId: taskId,
        taskName: args.taskName,
        scheduledTime: ISO8601DateFormatter().string(from: triggerDate),
        status: "Scheduled"
      )
      scheduledTasks[taskId] = taskInfo
      
      invoke.resolve([
        "taskId": taskId,
        "success": true,
        "message": "Task scheduled successfully"
      ])
    } catch {
      invoke.resolve([
        "taskId": taskId,
        "success": false,
        "message": "Failed to schedule task: \(error.localizedDescription)"
      ])
    }
  }
  
  @objc public func cancelTask(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(CancelTaskArgs.self)
    
    do {
      try cancelScheduledTask(taskId: args.taskId)
      if var taskInfo = scheduledTasks[args.taskId] {
        scheduledTasks[args.taskId] = ScheduledTaskInfo(
          taskId: taskInfo.taskId,
          taskName: taskInfo.taskName,
          scheduledTime: taskInfo.scheduledTime,
          status: "Cancelled"
        )
      }
      
      invoke.resolve([
        "success": true,
        "message": "Task cancelled successfully"
      ])
    } catch {
      invoke.resolve([
        "success": false,
        "message": "Failed to cancel task: \(error.localizedDescription)"
      ])
    }
  }
  
  @objc public func listTasks(_ invoke: Invoke) throws {
    let tasks = scheduledTasks.values.map { task in
      [
        "taskId": task.taskId,
        "taskName": task.taskName,
        "scheduledTime": task.scheduledTime,
        "status": task.status
      ]
    }
    
    invoke.resolve(["tasks": tasks])
  }
  
  private func requestNotificationPermission() {
    notificationCenter.requestAuthorization(options: [.alert, .sound, .badge]) { granted, error in
      if let error = error {
        print("Notification permission error: \(error)")
      }
    }
  }
  
  private func scheduleNotification(taskId: String, taskName: String, triggerDate: Date, parameters: [String: String]?) throws {
    let content = UNMutableNotificationContent()
    content.title = "Scheduled Task"
    content.body = "Running task: \(taskName)"
    content.sound = UNNotificationSound.default
    
    var userInfo: [String: Any] = [
      "taskId": taskId,
      "taskName": taskName,
      "isScheduledTask": true
    ]
    
    if let params = parameters {
      for (key, value) in params {
        userInfo["param_\(key)"] = value
      }
    }
    
    content.userInfo = userInfo
    
    let triggerDateComponents = Calendar.current.dateComponents([.year, .month, .day, .hour, .minute, .second], from: triggerDate)
    let trigger = UNCalendarNotificationTrigger(dateMatching: triggerDateComponents, repeats: false)
    
    let request = UNNotificationRequest(identifier: taskId, content: content, trigger: trigger)
    
    notificationCenter.add(request) { error in
      if let error = error {
        print("Failed to schedule notification: \(error)")
      }
    }
    
    // Also register background task for better reliability
    if #available(iOS 13.0, *) {
      registerBackgroundTask(taskId: taskId, taskName: taskName, triggerDate: triggerDate, parameters: parameters)
    }
  }
  
  @available(iOS 13.0, *)
  private func registerBackgroundTask(taskId: String, taskName: String, triggerDate: Date, parameters: [String: String]?) {
    let request = BGAppRefreshTaskRequest(identifier: "com.tauri.scheduled-task.\(taskId)")
    request.earliestBeginDate = triggerDate
    
    do {
      try BGTaskScheduler.shared.submit(request)
    } catch {
      print("Failed to register background task: \(error)")
    }
  }
  
  private func cancelScheduledTask(taskId: String) throws {
    // Cancel notification
    notificationCenter.removePendingNotificationRequests(withIdentifiers: [taskId])
    
    // Cancel background task
    if #available(iOS 13.0, *) {
      BGTaskScheduler.shared.cancel(taskRequestWithIdentifier: "com.tauri.scheduled-task.\(taskId)")
    }
  }
}

@_cdecl("init_plugin_schedule_task")
func initPlugin() -> Plugin {
  return ExamplePlugin()
}

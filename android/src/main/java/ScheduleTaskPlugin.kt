package com.plugin.scheduletask

import android.app.Activity
import android.app.AlarmManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.work.*
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Invoke
import java.util.*
import java.util.concurrent.TimeUnit

@InvokeArg
class PingArgs {
  var value: String? = null
}

@InvokeArg
class ScheduleTaskArgs {
  var taskName: String? = null
  var scheduleTime: ScheduleTimeArgs? = null
  var parameters: Map<String, String>? = null
}

@InvokeArg
class ScheduleTimeArgs {
  var dateTime: String? = null
  var duration: Long? = null
}

@InvokeArg
class CancelTaskArgs {
  var taskId: String? = null
}

@TauriPlugin
class ScheduleTaskPlugin(private val activity: Activity): Plugin(activity) {
    private val scheduledTasks = mutableMapOf<String, ScheduledTaskInfo>()

    @Command
    fun scheduleTask(invoke: Invoke) {
        val args = invoke.parseArgs(ScheduleTaskArgs::class.java)
        val taskName = args.taskName ?: return invoke.reject("Task name is required")
        val scheduleTime = args.scheduleTime ?: return invoke.reject("Schedule time is required")
        
        val taskId = UUID.randomUUID().toString()
        
        try {
            val delayMs = when {
                scheduleTime.dateTime != null -> {
                    val targetTime = parseDateTime(scheduleTime.dateTime!!)
                    maxOf(0, targetTime - System.currentTimeMillis())
                }
                scheduleTime.duration != null -> {
                    scheduleTime.duration!! * 1000
                }
                else -> return invoke.reject("Invalid schedule time")
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
                scheduleWithWorkManager(taskId, taskName, delayMs, args.parameters)
            } else {
                scheduleWithAlarmManager(taskId, taskName, delayMs, args.parameters)
            }

            val taskInfo = ScheduledTaskInfo(
                taskId = taskId,
                taskName = taskName,
                scheduledTime = (System.currentTimeMillis() + delayMs).toString(),
                status = "Scheduled"
            )
            scheduledTasks[taskId] = taskInfo

            val ret = JSObject()
            ret.put("taskId", taskId)
            ret.put("success", true)
            ret.put("message", "Task scheduled successfully")
            invoke.resolve(ret)
        } catch (e: Exception) {
            val ret = JSObject()
            ret.put("taskId", taskId)
            ret.put("success", false)
            ret.put("message", "Failed to schedule task: ${e.message}")
            invoke.resolve(ret)
        }
    }

    @Command
    fun cancelTask(invoke: Invoke) {
        val args = invoke.parseArgs(CancelTaskArgs::class.java)
        val taskId = args.taskId ?: return invoke.reject("Task ID is required")
        
        try {
            cancelScheduledTask(taskId)
            scheduledTasks[taskId]?.let { task ->
                scheduledTasks[taskId] = task.copy(status = "Cancelled")
            }

            val ret = JSObject()
            ret.put("success", true)
            ret.put("message", "Task cancelled successfully")
            invoke.resolve(ret)
        } catch (e: Exception) {
            val ret = JSObject()
            ret.put("success", false)
            ret.put("message", "Failed to cancel task: ${e.message}")
            invoke.resolve(ret)
        }
    }

    @Command
    fun listTasks(invoke: Invoke) {
        val tasks = scheduledTasks.values.map { task ->
            JSObject().apply {
                put("taskId", task.taskId)
                put("taskName", task.taskName)
                put("scheduledTime", task.scheduledTime)
                put("status", task.status)
            }
        }

        val ret = JSObject()
        ret.put("tasks", tasks)
        invoke.resolve(ret)
    }

    private fun scheduleWithWorkManager(taskId: String, taskName: String, delayMs: Long, parameters: Map<String, String>?) {
        val workData = Data.Builder()
            .putString("taskId", taskId)
            .putString("taskName", taskName)
            .putString("packageName", activity.packageName)
            
        parameters?.forEach { (key, value) ->
            workData.putString("param_$key", value)
        }

        val workRequest = OneTimeWorkRequestBuilder<ScheduledTaskWorker>()
            .setInitialDelay(delayMs, TimeUnit.MILLISECONDS)
            .setInputData(workData.build())
            .addTag(taskId)
            .build()

        WorkManager.getInstance(activity).enqueue(workRequest)
    }

    private fun scheduleWithAlarmManager(taskId: String, taskName: String, delayMs: Long, parameters: Map<String, String>?) {
        val alarmManager = activity.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        val intent = Intent(activity, ScheduledTaskReceiver::class.java).apply {
            putExtra("taskId", taskId)
            putExtra("taskName", taskName)
            putExtra("packageName", activity.packageName)
            parameters?.forEach { (key, value) ->
                putExtra("param_$key", value)
            }
        }

        val pendingIntent = PendingIntent.getBroadcast(
            activity,
            taskId.hashCode(),
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) PendingIntent.FLAG_IMMUTABLE else 0
        )

        val triggerTime = System.currentTimeMillis() + delayMs
        
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            alarmManager.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, triggerTime, pendingIntent)
        } else {
            alarmManager.setExact(AlarmManager.RTC_WAKEUP, triggerTime, pendingIntent)
        }
    }

    private fun cancelScheduledTask(taskId: String) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) {
            WorkManager.getInstance(activity).cancelAllWorkByTag(taskId)
        } else {
            val alarmManager = activity.getSystemService(Context.ALARM_SERVICE) as AlarmManager
            val intent = Intent(activity, ScheduledTaskReceiver::class.java)
            val pendingIntent = PendingIntent.getBroadcast(
                activity,
                taskId.hashCode(),
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) PendingIntent.FLAG_IMMUTABLE else 0
            )
            alarmManager.cancel(pendingIntent)
        }
    }

    private fun parseDateTime(dateTimeStr: String): Long {
        return try {
            val instant = java.time.Instant.parse(dateTimeStr)
            instant.toEpochMilli()
        } catch (e: Exception) {
            throw IllegalArgumentException("Invalid datetime format: $dateTimeStr")
        }
    }
}

data class ScheduledTaskInfo(
    val taskId: String,
    val taskName: String,
    val scheduledTime: String,
    val status: String
)

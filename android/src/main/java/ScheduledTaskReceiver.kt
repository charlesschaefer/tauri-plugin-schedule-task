package com.plugin.scheduletask

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import app.tauri.Logger

class ScheduledTaskReceiver : BroadcastReceiver() {
    
    override fun onReceive(context: Context, intent: Intent) {
        val taskId = intent.getStringExtra("taskId") ?: return
        val taskName = intent.getStringExtra("taskName") ?: return
        val packageName = intent.getStringExtra("packageName") ?: return
        
        executeScheduledTask(context, taskId, taskName, packageName, intent)
    }
    
    private fun executeScheduledTask(context: Context, taskId: String, taskName: String, packageName: String, originalIntent: Intent) {
        Logger.info("[RECEIVER] Creating the intent for $packageName.MainActivity")
        val launchIntent = Intent().apply {
            //setClassName(packageName, "$packageName.MainActivity")
            setClassName("com.plugin.scheduletask", "com.plugin.scheduletask.ScheduleTaskActivity")
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            putExtra("run_task", taskName)
            putExtra("task_id", taskId)
            
            // Copy task parameters
            for (key in originalIntent.extras?.keySet() ?: emptySet()) {
                if (key.startsWith("param_")) {
                    val paramName = key.removePrefix("param_")
                    val paramValue = originalIntent.getStringExtra(key)
                    putExtra("task_param_$paramName", paramValue)
                }
            }
        }
        
        Logger.info("[RECEIVER] We created the intent to run task $taskName with Id $taskId")
        Logger.info("[RECEIVER] Starting the activity intent")
        context.startActivity(launchIntent)
    }
}
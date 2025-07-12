package com.plugin.scheduletask

import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.work.Worker
import androidx.work.WorkerParameters
import app.tauri.Logger

class ScheduledTaskWorker(context: Context, params: WorkerParameters) : Worker(context, params) {
    
    override fun doWork(): Result {
        val taskId = inputData.getString("taskId") ?: return Result.failure()
        val taskName = inputData.getString("taskName") ?: return Result.failure()
        val packageName = inputData.getString("packageName") ?: return Result.failure()
        
        return try {
            executeScheduledTask(taskId, taskName, packageName)
            Logger.info("Executed the task $taskName successfully")
            Result.success()
        } catch (e: Exception) {
            Logger.error("Couldn't execute the task $taskName")
            Result.failure()
        }
    }
    
    private fun executeScheduledTask(taskId: String, taskName: String, packageName: String) {
        val intent = Intent().apply {
            setClassName(packageName, "$packageName.MainActivity")
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            putExtra("run_task", taskName)
            putExtra("task_id", taskId)

            Logger.info("Running task $taskName with Id $taskId")
            // Add task parameters
            for (key in inputData.keyValueMap.keys) {
                if (key.startsWith("param_")) {
                    val paramName = key.removePrefix("param_")
                    val paramValue = inputData.getString(key)
                    putExtra("task_param_$paramName", paramValue)
                    Logger.info("Param $paramName for the task $taskName: $paramValue")
                }
            }
        }
        
        applicationContext.startActivity(intent)
    }
}
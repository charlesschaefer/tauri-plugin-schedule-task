package com.plugin.scheduletask

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import app.tauri.Logger
import app.tauri.plugin.Channel
import app.tauri.plugin.JSObject


class ScheduleTaskActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val intent = intent
        val taskName = intent.getStringExtra("run_task")
        val taskId = intent.getStringExtra("task_id")

        // Extract parameters
        val params = mutableMapOf<String, String>()
        intent.extras?.keySet()?.forEach { key ->
            if (key.startsWith("task_param_")) {
                val paramName = key.removePrefix("task_param_")
                val paramValue = intent.getStringExtra(key)
                if (paramValue != null) {
                    params[paramName] = paramValue
                }
            }
        }

        // Compose event payload
        val payload = JSObject().apply {
            put("taskName", taskName)
            put("taskId", taskId)
            put("parameters", params)
        }

        // // Emit event to Tauri using Channel
        // Channel.emit("scheduledTask", payload)

        Logger.info("[ACTIVITY] ScheduleTaskActivity created with taskName: $taskName, taskId: $taskId, parameters: $params")
    }

    @JvmOverloads
    fun finishActivity() {

        Logger.info("[ACTIVITY] ScheduleTaskActivity finishing")
        finish()
    }
}
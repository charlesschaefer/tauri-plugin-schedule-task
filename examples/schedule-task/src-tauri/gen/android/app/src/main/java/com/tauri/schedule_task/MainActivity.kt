package com.tauri.schedule_task

import android.content.Intent
import app.tauri.Logger
import android.os.Bundle

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Logger.info("[ACTIVITY] App's MainActivity created")
    }

  override fun onStart() {
    super.onStart()
    Logger.info("[ACTIVITY] App's MainActivity started")
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    Logger.info("[ACTIVITY] App's MainActivity new intent: $intent")
  }
}
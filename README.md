# Tauri Plugin Schedule Task

This is a plugin to schedule tasks to run in the future. The task must be a rust code.

It works with tauri's async runtime in desktop implementation, and uses an Android plugin to schedule a worker. The worker will run even when the app is not on foreground.

This is important, because, in android, when the app is not in foreground, the JS can't be run because of webview throttling.


## What's the difference between Schedule Task and a Frontend setTimetout()?

To be honest, on desktop platforms not much. Schedule Task also use's Rust and Tauri's runtime to suspend a task for some amount of time before running it. 

It will be more useful for android platform, where the Frontend code won't execute if your app is minimized/running in background because of the webview throttling (that [can be deactivated](https://v2.tauri.app/reference/config/#backgroundthrottlingpolicy) for desktop but not for android).

# Usage

Check [Usage.md](Usage.md)


# Known Issues

Not really known, but I couldn't test the app in iOS or Mac environments. It must work as expected for Mac Desktop, but 
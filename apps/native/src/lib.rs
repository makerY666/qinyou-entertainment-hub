#[cfg(target_os = "android")]
mod android;
mod host;
#[cfg(not(test))]
use serde_json::Value;
#[cfg(not(test))]
use tauri::Manager;

#[cfg(target_os = "android")]
struct AndroidHost(tauri::plugin::PluginHandle<tauri::Wry>);

#[cfg(not(test))]
#[tauri::command]
async fn start_host(app: tauri::AppHandle, port: Option<u16>) -> Result<Value, String> {
    #[cfg(target_os = "android")]
    {
        return app
            .state::<AndroidHost>()
            .0
            .run_mobile_plugin(
                "startHost",
                serde_json::json!({"port":port.unwrap_or(18765)}),
            )
            .map_err(|e| e.to_string());
    }
    #[cfg(not(target_os = "android"))]
    {
        let path = app.path().app_data_dir().map_err(|e| e.to_string())?;
        tauri::async_runtime::spawn_blocking(move || host::start(path, port.unwrap_or(18765)))
            .await
            .map_err(|e| e.to_string())?
    }
}
#[cfg(not(test))]
#[tauri::command]
async fn stop_host(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _: Value = app
            .state::<AndroidHost>()
            .0
            .run_mobile_plugin("stopHost", serde_json::json!({}))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        tauri::async_runtime::spawn_blocking(host::stop)
            .await
            .map_err(|e| e.to_string())?
    }
}
#[cfg(not(test))]
#[tauri::command]
async fn host_info() -> Result<Option<Value>, String> {
    tauri::async_runtime::spawn_blocking(host::info)
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(not(test))]
#[tauri::command]
async fn set_table_orientation(app: tauri::AppHandle, landscape: bool) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _: Value = app
            .state::<AndroidHost>()
            .0
            .run_mobile_plugin(
                "setTableOrientation",
                serde_json::json!({"landscape": landscape}),
            )
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "android"))]
    let _ = (app, landscape);
    Ok(())
}

#[cfg(not(test))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "android")]
    let builder = builder.plugin(
        tauri::plugin::Builder::<tauri::Wry, ()>::new("host-service")
            .setup(|app, api| {
                let handle = api.register_android_plugin("cn.qinyou.hub", "HostPlugin")?;
                app.manage(AndroidHost(handle));
                Ok(())
            })
            .build(),
    );
    builder
        .invoke_handler(tauri::generate_handler![
            start_host,
            stop_host,
            host_info,
            set_table_orientation
        ])
        .setup(|app| {
            #[cfg(mobile)]
            let _ = app;
            #[cfg(desktop)]
            {
                use tauri::{
                    menu::{Menu, MenuItem},
                    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
                };
                let show = MenuItem::with_id(app, "show", "打开亲友娱乐 Hub", true, None::<&str>)?;
                let stop = MenuItem::with_id(app, "stop", "停止主机服务", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "退出并停止服务", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &stop, &quit])?;
                let icon = app
                    .default_window_icon()
                    .cloned()
                    .expect("bundled app icon");
                TrayIconBuilder::new()
                    .icon(icon)
                    .tooltip("亲友娱乐 Hub · 关闭窗口仍可开房")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "stop" => {
                            std::thread::spawn(|| {
                                let _ = host::stop();
                            });
                        }
                        "quit" => {
                            let app = app.clone();
                            std::thread::spawn(move || {
                                let _ = host::stop();
                                app.exit(0);
                            });
                        }
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            if let Some(w) = tray.app_handle().get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(desktop)]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
            #[cfg(mobile)]
            let _ = (window, event);
        })
        .run(tauri::generate_context!())
        .expect("无法启动亲友娱乐 Hub");
}

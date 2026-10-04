package cn.qinyou.hub

import android.Manifest
import android.app.Activity
import android.os.Build
import android.content.Intent
import android.content.pm.ActivityInfo
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.PermissionState
import app.tauri.plugin.Plugin

@InvokeArg
class StartArgs { var port: Int = 18765 }

@InvokeArg
class OrientationArgs { var landscape: Boolean = false }

@TauriPlugin(permissions = [
    Permission(strings = [Manifest.permission.POST_NOTIFICATIONS], alias = "notifications"),
    Permission(strings = ["android.permission.ACCESS_LOCAL_NETWORK"], alias = "localNetwork")
])
class HostPlugin(private val activity: Activity): Plugin(activity) {
    private var previousOrientation: Int? = null
    @Command
    fun setTableOrientation(invoke: Invoke) {
        val landscape = invoke.parseArgs(OrientationArgs::class.java).landscape
        activity.runOnUiThread {
            try {
                if (landscape) {
                    if (previousOrientation == null) previousOrientation = activity.requestedOrientation
                    activity.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
                } else {
                    previousOrientation?.let { activity.requestedOrientation = it }
                    previousOrientation = null
                }
                invoke.resolve()
            } catch (error: Exception) { invoke.reject(error.message ?: "无法调整牌桌方向") }
        }
    }
    @Command
    fun startHost(invoke: Invoke) {
        if (Build.VERSION.SDK_INT >= 37 && getPermissionState("localNetwork") != PermissionState.GRANTED) {
            requestPermissionForAlias("localNetwork", invoke, "networkPermissionResult")
            return
        }
        requestNotificationThenStart(invoke)
    }
    @PermissionCallback
    private fun networkPermissionResult(invoke: Invoke) {
        if (getPermissionState("localNetwork") != PermissionState.GRANTED) {
            invoke.reject("请允许访问本地网络，才能让同一 Wi-Fi 的亲友加入。")
            return
        }
        requestNotificationThenStart(invoke)
    }
    private fun requestNotificationThenStart(invoke: Invoke) {
        if (Build.VERSION.SDK_INT >= 33 && getPermissionState("notifications") == PermissionState.PROMPT) {
            requestPermissionForAlias("notifications", invoke, "notificationPermissionResult")
        } else { startService(invoke) }
    }
    @PermissionCallback
    private fun notificationPermissionResult(invoke: Invoke) { startService(invoke) }
    private fun startService(invoke: Invoke) {
        val port = invoke.parseArgs(StartArgs::class.java).port
        if (port !in 0..65535) { invoke.reject("端口超出范围"); return }
        HostService.start(activity, port) { result ->
            activity.runOnUiThread {
                try {
                    val json = JSObject(result)
                    if (json.optBoolean("ok")) invoke.resolve(JSObject(json.getJSONObject("info").toString()))
                    else invoke.reject(json.optString("error", "启动失败"))
                } catch (error: Exception) { invoke.reject(error.message ?: "无法读取主机状态") }
            }
        }
    }
    @Command
    fun stopHost(invoke: Invoke) {
        HostService.stop(activity) { result ->
            activity.runOnUiThread {
                val json = JSObject(result)
                if (json.optBoolean("ok")) invoke.resolve()
                else invoke.reject(json.optString("error", "停止失败"))
            }
        }
    }
}



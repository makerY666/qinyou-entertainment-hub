package cn.qinyou.hub

import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import org.json.JSONObject
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.Executors

/** Keeps the Rust Tokio runtime alive with the UI backgrounded or destroyed. */
class HostService : Service() {
    private var wakeLock: PowerManager.WakeLock? = null
    private var wifiLock: WifiManager.WifiLock? = null
    private val worker = Executors.newSingleThreadExecutor()
    override fun onBind(intent: Intent?): IBinder? = null
    override fun onCreate() {
        super.onCreate()
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel(CHANNEL, "局域网聚会服务", NotificationManager.IMPORTANCE_LOW))
    }
    private fun notification(text: String): Notification {
        val open = packageManager.getLaunchIntentForPackage(packageName) ?: Intent()
        val pending = PendingIntent.getActivity(this, 1, open, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        return Notification.Builder(this, CHANNEL)
            .setSmallIcon(android.R.drawable.ic_menu_myplaces)
            .setContentTitle("亲友娱乐 Hub 正在开房")
            .setContentText(text).setOngoing(true).setContentIntent(pending).build()
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val callbackId = intent?.getStringExtra("callback")
        val initialNotification = notification("同一 Wi-Fi 或热点中的亲友可通过浏览器加入")
        if (Build.VERSION.SDK_INT >= 29) startForeground(17, initialNotification, ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE)
        else startForeground(17, initialNotification)
        if (wakeLock == null) {
            wakeLock = (getSystemService(Context.POWER_SERVICE) as PowerManager)
                .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "QinyouHub:LocalGameHost").also { it.acquire() }
            @Suppress("DEPRECATION")
            wifiLock = (applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager)
                .createWifiLock(WifiManager.WIFI_MODE_FULL_HIGH_PERF, "QinyouHub:LocalGameHost").also { it.acquire() }
        }
        if (intent?.action == "cn.qinyou.hub.STOP") {
            worker.execute {
                val reply = try { NativeBridge.stop() }
                catch (e: Throwable) { JSONObject().put("ok", false).put("error", e.message).toString() }
                callbackId?.let { callbacks.remove(it)?.invoke(reply) }
                stopSelf(startId)
            }
            return START_NOT_STICKY
        }
        // START_NOT_STICKY: restore needs a visible, explicit user action; no secret background restart.
        worker.execute {
            val reply = try { NativeBridge.start(filesDir.resolve("host").absolutePath, intent?.getIntExtra("port", 18765) ?: 18765) }
            catch (e: Throwable) { JSONObject().put("ok", false).put("error", e.message ?: "主机启动失败").toString() }
            callbackId?.let { callbacks.remove(it)?.invoke(reply) }
            if (!JSONObject(reply).optBoolean("ok")) { stopSelf(startId) }
            else {
                val port = JSONObject(reply).getJSONObject("info").getInt("port")
                getSystemService(NotificationManager::class.java).notify(17, notification("主机端口 $port · 点击返回牌桌 · 服务持续运行"))
            }
        }
        return START_NOT_STICKY
    }
    override fun onDestroy() {
        worker.execute { try { NativeBridge.stop() } catch (_: Throwable) {} }
        worker.shutdown()
        wifiLock?.let { if (it.isHeld) it.release() }
        wakeLock?.let { if (it.isHeld) it.release() }
        wifiLock = null; wakeLock = null
        super.onDestroy()
    }
    companion object {
        private const val CHANNEL = "qinyou-host"
        private val callbacks = ConcurrentHashMap<String, (String) -> Unit>()
        fun start(activity: Activity, port: Int, callback: (String) -> Unit) {
            val id = UUID.randomUUID().toString()
            callbacks[id] = callback
            try { activity.startForegroundService(Intent(activity, HostService::class.java).putExtra("port", port).putExtra("callback", id)) }
            catch (e: Exception) { callbacks.remove(id)?.invoke(JSONObject().put("ok", false).put("error", e.message).toString()) }
        }
        fun stop(activity: Activity, callback: (String) -> Unit) {
            val id = UUID.randomUUID().toString()
            callbacks[id] = callback
            try { activity.startForegroundService(Intent(activity, HostService::class.java).setAction("cn.qinyou.hub.STOP").putExtra("callback", id)) }
            catch (e: Exception) { callbacks.remove(id)?.invoke(JSONObject().put("ok", false).put("error", e.message).toString()) }
        }
    }
}




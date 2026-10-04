package cn.qinyou.hub

/** Loaded by the Android foreground service, independent of Activity/WebView. */
object NativeBridge {
    init { System.loadLibrary("hub_native_lib") }
    @JvmStatic external fun start(dataDir: String, port: Int): String
    @JvmStatic external fun stop(): String
}

# JNI symbols and Tauri reflection must retain the exact names in release builds.
-keep class cn.qinyou.hub.NativeBridge { *; }
-keep class cn.qinyou.hub.HostPlugin { *; }
-keep class cn.qinyou.hub.StartArgs { *; }
-keep class cn.qinyou.hub.HostService { *; }
-keep class cn.qinyou.hub.OrientationArgs { *; }

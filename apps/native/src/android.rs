use jni::{
    objects::{JClass, JString},
    sys::{jint, jstring},
    JNIEnv,
};
use serde_json::json;
use std::path::PathBuf;

// JNI names match NativeBridge.kt. Service owns calls and outlives the WebView.
#[no_mangle]
pub extern "system" fn Java_cn_qinyou_hub_NativeBridge_start(
    mut env: JNIEnv,
    _: JClass,
    path: JString,
    port: jint,
) -> jstring {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path: String = env.get_string(&path).map_err(|e| e.to_string())?.into();
        if !(0..=65535).contains(&port) {
            return Err("端口必须在 0 到 65535 之间".into());
        }
        crate::host::start(PathBuf::from(path), port as u16)
    }))
    .unwrap_or_else(|_| Err("主机启动异常".into()));
    let value = match result {
        Ok(info) => json!({"ok":true,"info":info}),
        Err(error) => json!({"ok":false,"error":error}),
    };
    env.new_string(value.to_string())
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}
#[no_mangle]
pub extern "system" fn Java_cn_qinyou_hub_NativeBridge_stop(env: JNIEnv, _: JClass) -> jstring {
    let result =
        std::panic::catch_unwind(crate::host::stop).unwrap_or_else(|_| Err("主机停止异常".into()));
    let value = match result {
        Ok(()) => json!({"ok":true}),
        Err(error) => json!({"ok":false,"error":error}),
    };
    env.new_string(value.to_string())
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

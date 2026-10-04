// Build helper for Windows toolchains installed beneath non-ASCII workspace paths.
// Cargo omits target RUSTFLAGS for host build scripts during cross compilation.
fn main() {
    let mut args = std::env::args_os().skip(1);
    let compiler = args.next().expect("Cargo must pass its compiler");
    let mut args: Vec<_> = args.collect();
    if !args.iter().any(|a| a == "--sysroot") {
        args.push("--sysroot".into());
        args.push(std::env::var_os("HUB_RUST_SYSROOT").expect("HUB_RUST_SYSROOT"));
    }
    if args.iter().any(|a| a == "aarch64-linux-android") {
        for (key, flag) in [
            ("HUB_ANDROID_SYSROOT", "--sysroot="),
            ("HUB_ANDROID_RESOURCE", "-resource-dir="),
        ] {
            if let Some(path) = std::env::var_os(key) {
                args.push("-C".into());
                args.push(format!("link-arg={flag}{}", path.to_string_lossy()).into());
            }
        }
    }
    let status = std::process::Command::new(compiler)
        .args(args)
        .status()
        .expect("launch rustc");
    std::process::exit(status.code().unwrap_or(1));
}

param([ValidateSet('dev','windows','android','android-init','check')][string]$Target='check')
$ErrorActionPreference='Stop'
$repo = Split-Path $PSScriptRoot -Parent
Push-Location $repo
try {
    if (Test-Path "$repo/.tools/cargo/bin") { $env:CARGO_HOME="$repo/.tools/cargo"; $env:RUSTUP_HOME="$repo/.tools/rustup"; $env:Path="$repo/.tools/cargo/bin;$env:Path" }
    if (Test-Path "$repo/.tools/android") { $env:ANDROID_HOME="$repo/.tools/android"; $env:ANDROID_SDK_ROOT=$env:ANDROID_HOME; $env:NDK_HOME="$repo/.tools/android/ndk/29.0.14206865" }
    $localJdk=Get-ChildItem "$repo/.tools/android/jdk" -Directory -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($localJdk) { $env:JAVA_HOME=$localJdk.FullName }
    if ($env:JAVA_HOME) { $env:Path="$env:JAVA_HOME/bin;$env:Path" }
    $cli = "$repo/node_modules/.bin/tauri.cmd"
    if (!(Test-Path $cli)) { $cli="$repo/.tools/tauri-cli/node_modules/.bin/tauri.cmd" }
    if (!(Test-Path $cli)) { throw '请先执行 npm install（需 @tauri-apps/cli 2）。' }
    Push-Location "$repo/apps/native"
    try {
        switch ($Target) {
            'check' { & cargo check -p hub-native }
            'dev' { & $cli dev }
            'windows' { & $cli build --bundles nsis }
            'android-init' { & $cli android init --ci --skip-targets-install; if ($LASTEXITCODE -ne 0) { throw 'Android 初始化失败' }; & "$PSScriptRoot/native-sync-android.ps1" }
            'android' {
                if (!(Test-Path gen/android/app/build.gradle.kts)) { throw '先运行 scripts/native.ps1 android-init。' }
                & "$PSScriptRoot/native-sync-android.ps1"
                & $cli android build --target aarch64 --apk
            }
        }
        if ($LASTEXITCODE -ne 0) { throw "原生构建失败，退出码 $LASTEXITCODE" }
    } finally { Pop-Location }
} finally { Pop-Location }


#requires -Version 7.1
param([ValidateSet('windows','android','both')][string]$Target='both')
$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot -Parent
$hash=[Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($repo))).Substring(0,10)
$buildRoot=Join-Path $env:LOCALAPPDATA "QinyouHubBuild/native-$hash"
if($buildRoot -match '[^\x00-\x7F]'){throw '请在英文用户路径下构建；本地工具链不支持中文编译输出路径。'}
$stage=Join-Path $buildRoot 'source'
$alias=Join-Path $buildRoot 'original'
New-Item -ItemType Directory -Force $buildRoot,$stage | Out-Null
if(!(Test-Path -LiteralPath $alias)){New-Item -ItemType Junction -Path $alias -Value $repo | Out-Null}
$link=Get-Item -LiteralPath $alias
if(!$link.LinkType -or @($link.Target)[0].TrimEnd('\') -ne $repo.TrimEnd('\')){throw '构建来源链接与项目不符。'}
function Run-Checked([string]$Program,[string[]]$Arguments){& $Program @Arguments; if($LASTEXITCODE -ne 0){throw "$Program 失败：$LASTEXITCODE"}}
function Copy-Tree([string]$From,[string]$To){& robocopy $From $To /E /XD node_modules target .gradle build .git /NFL /NDL /NJH /NJS /NP | Out-Null; if($LASTEXITCODE -gt 7){throw "复制构建源失败：$From"}}
foreach($name in @('Cargo.toml','Cargo.lock','package.json','package-lock.json')){if(Test-Path "$repo/$name"){Copy-Item -LiteralPath "$repo/$name" -Destination "$stage/$name" -Force}}
foreach($name in @('crates','apps/native','web','scripts')){Copy-Tree "$repo/$name" "$stage/$name"}
foreach($dir in @('node_modules','web/node_modules')){
    if(!(Test-Path -LiteralPath "$repo/$dir")){throw "缺少 $dir，请先执行 npm install 和 npm --prefix web install。"}
    if(!(Test-Path -LiteralPath "$stage/$dir")){New-Item -ItemType Junction -Path "$stage/$dir" -Value "$repo/$dir" | Out-Null}
}
$env:CARGO_TARGET_DIR=Join-Path $buildRoot 'target'
$localRust="$alias/.tools/rustup/toolchains/stable-x86_64-pc-windows-gnu"
if(Test-Path -LiteralPath $localRust){
    $env:CARGO_HOME="$alias/.tools/cargo"; $env:RUSTUP_HOME="$alias/.tools/rustup"
    $env:Path="$alias/.tools/cargo/bin;$env:Path"
    $gnu='D:/Program/CLion 2026.1/bin/mingw/bin'
    if(Test-Path -LiteralPath $gnu){$env:Path="$gnu;$env:Path"}
    $env:HUB_RUST_SYSROOT=$localRust
    $wrapper=Join-Path $buildRoot 'rustc-wrapper.exe'
    Run-Checked 'rustc' @('--sysroot',$localRust,"$PSScriptRoot/native-rustc-wrapper.rs",'-o',$wrapper)
    $env:RUSTC_WRAPPER=$wrapper
    $env:CARGO_ENCODED_RUSTFLAGS='--sysroot'+[char]31+$localRust
}
if(Test-Path "$alias/.tools/android"){
    $env:ANDROID_HOME="$alias/.tools/android"; $env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
    $env:NDK_HOME="$env:ANDROID_HOME/ndk/29.0.14206865"
    $env:GRADLE_USER_HOME="$alias/.tools/gradle"
}
if(!$env:JAVA_HOME -or $env:JAVA_HOME -like '*CLion*'){
    if(Test-Path 'D:/Program/PyCharm 2025.3.4/jbr/bin/java.exe'){$env:JAVA_HOME='D:/Program/PyCharm 2025.3.4/jbr'}
}
if($env:NDK_HOME){
    $llvm="$env:NDK_HOME/toolchains/llvm/prebuilt/windows-x86_64"
    $env:HUB_ANDROID_SYSROOT="$llvm/sysroot"
    $clang=Get-ChildItem "$llvm/lib/clang" -Directory | Sort-Object Name -Descending | Select-Object -First 1
    $env:HUB_ANDROID_RESOURCE=$clang.FullName
}
Run-Checked 'npm.cmd' @('--prefix',"$stage/web",'run','build')
$override=Join-Path $buildRoot 'build-config.json'
'{"build":{"beforeBuildCommand":""}}' | Set-Content -Encoding utf8 $override
$cli="$stage/node_modules/.bin/tauri.cmd"
$out=Join-Path $repo 'artifacts'
New-Item -ItemType Directory -Force $out | Out-Null
Push-Location "$stage/apps/native"
try {
    if($Target -in @('windows','both')){
        Run-Checked $cli @('build','--bundles','nsis','--ci','--config',$override)
        $installer=Get-ChildItem "$env:CARGO_TARGET_DIR/release/bundle/nsis/*-setup.exe" | Select-Object -First 1
        if(!$installer){throw 'NSIS 未产生安装程序。'}
        Copy-Item -LiteralPath $installer.FullName -Destination "$out/qinyou-hub-windows-x64-setup.exe" -Force
    }
    if($Target -in @('android','both')){
        if(!(Test-Path "$stage/apps/native/gen/android/app/src/main/AndroidManifest.xml")){
            Run-Checked $cli @('android','init','--ci','--skip-targets-install')
        }
        & "$stage/scripts/native-sync-android.ps1"
        $log=Join-Path $buildRoot 'android-build.log'
        & $cli android build --debug --target aarch64 --apk --ci --config $override *> $log
        $result=$LASTEXITCODE
        $text=Get-Content -Raw $log
        if($result -ne 0 -and !($text -match 'Finished `dev` profile' -and $text -match 'Creation symbolic link is not allowed')){Get-Content $log -Tail 50; throw 'Android 编译失败；详见上方日志。'}
        # Rust already built successfully. Copying is supported even where Windows denies symlinks.
        $jni="$stage/apps/native/gen/android/app/src/main/jniLibs/arm64-v8a"
        New-Item -ItemType Directory -Force $jni | Out-Null
        $so="$jni/libhub_native_lib.so"
        if((Get-Item -LiteralPath $so -ErrorAction SilentlyContinue).LinkType){Remove-Item -LiteralPath $so}
        Copy-Item -LiteralPath "$env:CARGO_TARGET_DIR/aarch64-linux-android/debug/libhub_native_lib.so" -Destination $so -Force
        Run-Checked "$llvm/bin/llvm-strip.exe" @('--strip-debug',$so)
        Run-Checked "$stage/apps/native/gen/android/gradlew.bat" @('-p',"$stage/apps/native/gen/android",'assembleArm64Debug','-x','rustBuildArm64Debug','-Pkotlin.incremental=false','--no-daemon')
        $apk="$stage/apps/native/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk"
        $aligned=Join-Path $buildRoot 'aligned-preview.apk'
        $buildTools="$env:ANDROID_HOME/build-tools/37.0.0"
        Run-Checked "$buildTools/zipalign.exe" @('-P','16','-f','4',$apk,$aligned)
        $androidUser=if($env:ANDROID_USER_HOME){$env:ANDROID_USER_HOME}else{Join-Path $env:USERPROFILE '.android'}
        Run-Checked "$buildTools/apksigner.bat" @('sign','--ks',"$androidUser/debug.keystore",'--ks-key-alias','androiddebugkey','--ks-pass','pass:android','--key-pass','pass:android','--out',"$out/qinyou-hub-android-arm64-preview.apk",$aligned)
        Run-Checked "$buildTools/apksigner.bat" @('verify','--verbose',"$out/qinyou-hub-android-arm64-preview.apk")
    }
} finally {Pop-Location}
Get-ChildItem "$out/qinyou-hub-*" | Get-FileHash -Algorithm SHA256 | Format-Table -AutoSize
Write-Host "已输出到 $out。安卓为调试签名预览包；真机热点、锁屏和双厂商兼容性仍须验收。"



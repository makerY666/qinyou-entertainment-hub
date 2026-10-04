$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot -Parent
$android=Join-Path $repo 'apps/native/gen/android'
if (!(Test-Path "$android/app/src/main/AndroidManifest.xml")) { throw '请先运行 Tauri android init。' }
$dest="$android/app/src/main/java/cn/qinyou/hub"
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item -Path "$repo/apps/native/android-src/*.kt" -Destination $dest -Force
[xml]$manifest=Get-Content -Raw "$android/app/src/main/AndroidManifest.xml"
$ns='http://schemas.android.com/apk/res/android'
$permissions=@('android.permission.INTERNET','android.permission.ACCESS_NETWORK_STATE','android.permission.ACCESS_WIFI_STATE','android.permission.CHANGE_NETWORK_STATE','android.permission.CHANGE_WIFI_STATE','android.permission.WAKE_LOCK','android.permission.FOREGROUND_SERVICE','android.permission.FOREGROUND_SERVICE_CONNECTED_DEVICE','android.permission.POST_NOTIFICATIONS','android.permission.ACCESS_LOCAL_NETWORK')
foreach ($permission in $permissions) {
    $found=@($manifest.manifest.'uses-permission' | Where-Object { $_.GetAttribute('name',$ns) -eq $permission })
    if ($found.Count -eq 0) { $node=$manifest.CreateElement('uses-permission'); [void]$node.SetAttribute('name',$ns,$permission); $manifest.manifest.AppendChild($node) | Out-Null }
}
$app=$manifest.manifest.application
[void]$app.SetAttribute('usesCleartextTraffic',$ns,'true')
[void]$app.SetAttribute('allowBackup',$ns,'false')
$service=@($app.service | Where-Object { $_ -and $_.GetAttribute('name',$ns) -eq 'cn.qinyou.hub.HostService' })
if ($service.Count -eq 0) {
    $service=$manifest.CreateElement('service'); [void]$service.SetAttribute('name',$ns,'cn.qinyou.hub.HostService'); $app.AppendChild($service) | Out-Null
} else {
    $service=$service[0]
}
[void]$service.SetAttribute('exported',$ns,'false')
[void]$service.SetAttribute('foregroundServiceType',$ns,'connectedDevice')
[void]$service.SetAttribute('stopWithTask',$ns,'false')
$manifest.Save("$android/app/src/main/AndroidManifest.xml")
$gradle=Get-Content -Raw "$android/app/build.gradle.kts"
$gradle=$gradle -replace 'compileSdk\s*=\s*\d+', 'compileSdk = 37' -replace 'targetSdk\s*=\s*\d+', 'targetSdk = 37' -replace 'minSdk\s*=\s*\d+', 'minSdk = 28'
Set-Content -Encoding utf8 "$android/app/build.gradle.kts" $gradle.TrimEnd()
Write-Host '已同步原生前台服务、JNI 桥和 Android 权限。'
$taskPath="$android/buildSrc/src/main/java/cn/qinyou/hub/kotlin/BuildTask.kt"
if (Test-Path $taskPath) { (Get-Content -Raw $taskPath).Replace('val executable = """node""";', 'val executable = """npx""";').TrimEnd() | Set-Content -Encoding utf8 $taskPath }
Copy-Item -LiteralPath "$repo/apps/native/android-src/host-service.pro" -Destination "$android/app/host-service.pro" -Force

Copy-Item -Path "$repo/apps/native/icons/android/*" -Destination "$android/app/src/main/res" -Recurse -Force
$wrapper="$android/gradle/wrapper/gradle-wrapper.properties"
if ((Select-String -Quiet -Path $wrapper -Pattern 'gradle-9.6.1-bin.zip') -and !(Select-String -Quiet -Path $wrapper -Pattern 'distributionSha256Sum')) { Add-Content -Encoding ascii $wrapper 'distributionSha256Sum=9c0f7faeeb306cb14e4279a3e084ca6b596894089a0638e68a07c945a32c9e14' }

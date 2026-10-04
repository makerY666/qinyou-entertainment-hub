#requires -Version 7.1
param(
    [ValidateSet('windows','android','both')][string]$Target='both',
    [string]$FrontendEntry,
    [string]$ArtifactDirectory
)
$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot -Parent
if(!$ArtifactDirectory){$ArtifactDirectory=Join-Path $repo 'artifacts'}
if(!$FrontendEntry){
    $html=Get-Content -Raw "$repo/web/dist/index.html"
    $FrontendEntry=[regex]::Match($html,'index-[A-Za-z0-9_-]+\.js').Value
}
if(!$FrontendEntry){throw '无法确定前端入口，请先构建网页或指定 -FrontendEntry。'}
function Assert-EmbeddedEntry([IO.Stream]$Stream,[string]$Label){
    $buffer=[byte[]]::new(65536)
    $tail=''
    while(($count=$Stream.Read($buffer,0,$buffer.Length)) -gt 0){
        $chunk=$tail+[Text.Encoding]::ASCII.GetString($buffer,0,$count)
        if($chunk.Contains($FrontendEntry)){return}
        $tail=$chunk.Substring([Math]::Max(0,$chunk.Length-$FrontendEntry.Length))
    }
    throw "$Label 未包含预期前端入口 $FrontendEntry，可能是旧包。"
}
$records=@()
if($Target -in @('windows','both')){
    $installer=Join-Path $ArtifactDirectory 'qinyou-hub-windows-x64-setup.exe'
    $sevenZip=(Get-Command '7z.exe' -ErrorAction SilentlyContinue).Source
    if(!$sevenZip -and (Test-Path 'C:/Program Files/7-Zip/7z.exe')){$sevenZip='C:/Program Files/7-Zip/7z.exe'}
    if(!$sevenZip){throw '验证 NSIS 内容需要 7-Zip（7z.exe）。'}
    $listing=& $sevenZip l $installer
    if($LASTEXITCODE -ne 0){throw '无法读取 NSIS 安装包。'}
    foreach($required in @('WebView2Loader.dll','MicrosoftEdgeWebView2RuntimeInstaller.exe','hub-native.exe')){
        if(!($listing -match [regex]::Escape($required))){throw "安装包缺少 $required"}
    }
    $temp=Join-Path ([IO.Path]::GetTempPath()) ('qinyou-package-check-'+[guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory $temp | Out-Null
    try {
        & $sevenZip e $installer 'hub-native.exe' "-o$temp" '-y' | Out-Null
        if($LASTEXITCODE -ne 0){throw '无法提取安装包内的应用。'}
        $stream=[IO.File]::OpenRead("$temp/hub-native.exe")
        try {Assert-EmbeddedEntry $stream 'NSIS 应用'} finally {$stream.Dispose()}
    } finally {
        # Delete only files created in this unique verification directory.
        if(Test-Path -LiteralPath "$temp/hub-native.exe"){Remove-Item -LiteralPath "$temp/hub-native.exe"}
        Remove-Item -LiteralPath $temp
    }
    $records+=@{file=[IO.Path]::GetFileName($installer);sha256=(Get-FileHash $installer).Hash;bytes=(Get-Item $installer).Length;frontendEntry=$FrontendEntry;webView2Loader=$true;offlineWebView2Installer=$true}
}
if($Target -in @('android','both')){
    $apk=Join-Path $ArtifactDirectory 'qinyou-hub-android-arm64-preview.apk'
    $zip=[IO.Compression.ZipFile]::OpenRead($apk)
    try {
        $entry=$zip.GetEntry('lib/arm64-v8a/libhub_native_lib.so')
        if(!$entry){throw 'APK 缺少 ARM64 原生库。'}
        $stream=$entry.Open()
        try {Assert-EmbeddedEntry $stream 'APK 原生库'} finally {$stream.Dispose()}
    } finally {$zip.Dispose()}
    $records+=@{file=[IO.Path]::GetFileName($apk);sha256=(Get-FileHash $apk).Hash;bytes=(Get-Item $apk).Length;frontendEntry=$FrontendEntry;signing='Android debug preview'}
}
$result=@{verifiedAt=[DateTime]::UtcNow.ToString('o');artifacts=$records;realDeviceAcceptance=$false}
$result | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $ArtifactDirectory "native-$Target-verification.json")
$result | ConvertTo-Json -Depth 5

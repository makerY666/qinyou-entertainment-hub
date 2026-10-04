param([ValidateSet('host','test','check','web','build')][string]$Action='host',[int]$Port=18765)
$ErrorActionPreference='Stop'
$projectRoot=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
if(Test-Path -LiteralPath (Join-Path $projectRoot '.tools/cargo/bin/cargo.exe')) {
    $env:CARGO_HOME=Join-Path $projectRoot '.tools/cargo'
    $env:RUSTUP_HOME=Join-Path $projectRoot '.tools/rustup'
    $env:PATH="$(Join-Path $env:CARGO_HOME 'bin');$env:PATH"
    # GNU ld cannot read Chinese paths in response files. Keep source in place,
    # expose an ASCII junction for its sysroot and use a physical ASCII target.
    if(Test-Path -LiteralPath (Join-Path $projectRoot '.tools/rustup/toolchains/stable-x86_64-pc-windows-gnu')) {
        $buildRoot=Join-Path $env:LOCALAPPDATA 'QinyouHubBuild'
        New-Item -ItemType Directory -Path $buildRoot -Force | Out-Null
        $sourceLink=Join-Path $buildRoot 'source'
        if(!(Test-Path -LiteralPath $sourceLink)) {New-Item -ItemType Junction -Path $sourceLink -Value $projectRoot | Out-Null}
        $link=Get-Item -LiteralPath $sourceLink
        if(!$link.LinkType -or @($link.Target)[0].TrimEnd('\') -ne $projectRoot.TrimEnd('\')) {throw '构建目录已被另一项目使用，请使用标准 MSVC 工具链或更换 QinyouHubBuild 目录。'}
        $env:CARGO_HOME=Join-Path $sourceLink '.tools/cargo'
        $env:RUSTUP_HOME=Join-Path $sourceLink '.tools/rustup'
        $env:CARGO_TARGET_DIR=Join-Path $buildRoot 'target'
        $env:CARGO_ENCODED_RUSTFLAGS='--sysroot'+[char]31+(Join-Path $env:RUSTUP_HOME 'toolchains/stable-x86_64-pc-windows-gnu')
        $env:PATH="$(Join-Path $env:CARGO_HOME 'bin');$env:PATH"
    }
}
$localGnu='D:\Program\CLion 2026.1\bin\mingw\bin'
if(Test-Path -LiteralPath $localGnu){$env:PATH="$localGnu;$env:PATH"}
switch($Action){
    'web' {npm.cmd --prefix web run dev}
    'host' {npm.cmd --prefix web run build; if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}; cargo run -p hub-server -- --port $Port}
    'test' {cargo test -p mahjong -p doudizhu -p guandan -p hub-server}
    'check' {npm.cmd --prefix web run build; if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}; cargo check --workspace}
    'build' {npm.cmd --prefix web run build; if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}; cargo build -p hub-server --release}
}
exit $LASTEXITCODE

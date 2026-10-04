# Offline voice assets are shipped with the app; Windows TTS is only a build tool.
param([string]$Voice='Microsoft Huihui Desktop')
$ErrorActionPreference='Stop'
$repo=Split-Path $PSScriptRoot -Parent
$output=Join-Path $repo 'web/public/audio'
$scratch=Join-Path $repo '.data/audio-build'
New-Item -ItemType Directory -Force "$output/voice",$scratch | Out-Null
Add-Type -AssemblyName System.Speech
$speaker=[System.Speech.Synthesis.SpeechSynthesizer]::new()
$speaker.SelectVoice($Voice)
$speaker.Rate=1
$speaker.Volume=90
$names=@('一','二','三','四','五','六','七','八','九')
$suits=@('万','条','筒')
$clips=[ordered]@{}
for($s=0;$s -lt 3;$s++) { for($n=0;$n -lt 9;$n++) { $clips["tile-$($s*9+$n)"]="$($names[$n])$($suits[$s])" } }
$clips['peng']='碰'
$clips['kong']='杠'
$clips['hu']='胡牌'
$clips['zimo']='自摸'
$clips['your-turn']='轮到你出牌'
$clips['dingque']='请选择定缺'
foreach($key in $clips.Keys) {
    $wav=Join-Path $scratch "$key.wav"
    $speaker.SetOutputToWaveFile($wav)
    $speaker.Speak($clips[$key])
    $speaker.SetOutputToNull()
    & ffmpeg -hide_banner -loglevel error -y -i $wav -af 'silenceremove=start_periods=1:start_threshold=-45dB:stop_periods=-1:stop_duration=0.15:stop_threshold=-45dB,afade=t=in:d=0.01,loudnorm=I=-18:TP=-2:LRA=7' -ar 24000 -ac 1 -codec:a libmp3lame -b:a 64k "$output/voice/$key.mp3"
    if($LASTEXITCODE -ne 0){throw "语音编码失败：$key"}
}
$speaker.Dispose()
& python "$PSScriptRoot/generate-bgm.py" "$scratch/bgm.wav"
if($LASTEXITCODE -ne 0){throw '背景音乐生成失败'}
& ffmpeg -hide_banner -loglevel error -y -i "$scratch/bgm.wav" -ar 32000 -ac 2 -codec:a libmp3lame -b:a 128k "$output/bgm.mp3"
if($LASTEXITCODE -ne 0){throw '背景音乐编码失败'}
Write-Output "Generated $($clips.Count) voice clips and one original music loop."

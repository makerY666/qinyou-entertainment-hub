# Offline Mandarin voice pack. Generated with the same voice and mix as the other games.
param([string]$Voice='Microsoft Huihui Desktop')
$ErrorActionPreference='Stop'
$gdRepo=Split-Path $PSScriptRoot -Parent
$gdOutput=Join-Path $gdRepo 'web/public/audio/voice/gd'
$gdScratch=Join-Path $gdRepo '.data/guandan-audio-build'
New-Item -ItemType Directory -Force $gdOutput,$gdScratch | Out-Null
Add-Type -AssemblyName System.Speech
$gdSpeaker=[System.Speech.Synthesis.SpeechSynthesizer]::new()
$gdSpeaker.SelectVoice($Voice)
$gdSpeaker.Rate=1
$gdSpeaker.Volume=90
$gdRanks=@('三','四','五','六','七','八','九','十','勾','圈','凯','尖','二','小王','大王')
$gdClips=[ordered]@{}
for($i=0;$i -lt 15;$i++) { $gdClips["single-$i"]=$gdRanks[$i]; $gdClips["pair-$i"]="对$($gdRanks[$i])" }
for($i=0;$i -lt 13;$i++) { $gdClips["triple-$i"]="三个$($gdRanks[$i])" }
$gdClips['triple_pair']='三带两'
$gdClips['straight']='顺子'
$gdClips['pair_straight']='三连对'
$gdClips['airplane']='钢板'
$gdClips['bomb']='炸弹'
$gdClips['straight_flush']='同花顺'
$gdClips['rocket']='四大天王'
$gdClips['pass']='不出'
$gdClips['one-left']='只剩一张牌了'
$gdClips['two-left']='只剩两张牌了'
$gdClips['deal']='开始发牌'
$gdClips['your-turn']='轮到你出牌'
$gdClips['your-return']='请还贡'
$gdClips['tribute']='进贡'
$gdClips['return']='还贡'
$gdClips['anti-tribute']='抗贡'
$gdClips['wind']='对家接风'
$gdClips['first']='头游'
$gdClips['out']='出完了'
$gdClips['team-win']='本队获胜'
$gdClips['match-win']='打过尖，本轮获胜'
try {
    foreach($key in $gdClips.Keys) {
        $gdWav=Join-Path $gdScratch "$key.wav"
        $gdSpeaker.SetOutputToWaveFile($gdWav)
        $gdSpeaker.Speak($gdClips[$key])
        $gdSpeaker.SetOutputToNull()
        & ffmpeg -hide_banner -loglevel error -y -i $gdWav -af 'silenceremove=start_periods=1:start_threshold=-45dB:stop_periods=-1:stop_duration=0.12:stop_threshold=-45dB,afade=t=in:d=0.01,loudnorm=I=-18:TP=-2:LRA=7' -ar 24000 -ac 1 -codec:a libmp3lame -b:a 64k (Join-Path $gdOutput "$key.mp3")
        if($LASTEXITCODE -ne 0){throw "Voice encoding failed: $key"}
    }
} finally { $gdSpeaker.Dispose() }
$gdClips | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $gdOutput 'manifest.json') -Encoding UTF8
Write-Output "Generated $($gdClips.Count) offline Guandan voice clips."

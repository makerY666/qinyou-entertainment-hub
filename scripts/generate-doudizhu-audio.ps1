# Offline Mandarin voice pack. Runtime playback never needs TTS or the Internet.
param([string]$Voice='Microsoft Huihui Desktop')
$ErrorActionPreference='Stop'
$ddzRepo=Split-Path $PSScriptRoot -Parent
$ddzOutput=Join-Path $ddzRepo 'web/public/audio/voice/ddz'
$ddzScratch=Join-Path $ddzRepo '.data/doudizhu-audio-build'
New-Item -ItemType Directory -Force $ddzOutput,$ddzScratch | Out-Null
Add-Type -AssemblyName System.Speech
$ddzSpeaker=[System.Speech.Synthesis.SpeechSynthesizer]::new()
$ddzSpeaker.SelectVoice($Voice)
$ddzSpeaker.Rate=1
$ddzSpeaker.Volume=90
$ddzRanks=@('三','四','五','六','七','八','九','十','勾','圈','凯','尖','二','小王','大王')
$ddzClips=[ordered]@{}
for($i=0;$i -lt 15;$i++) { $ddzClips["single-$i"]=$ddzRanks[$i] }
for($i=0;$i -lt 13;$i++) { $ddzClips["pair-$i"]="对$($ddzRanks[$i])"; $ddzClips["triple-$i"]="三个$($ddzRanks[$i])" }
$ddzClips['triple_single']='三带一'
$ddzClips['triple_pair']='三带一对'
$ddzClips['straight']='顺子'
$ddzClips['pair_straight']='连对'
$ddzClips['airplane']='飞机'
$ddzClips['airplane_single']='飞机带单'
$ddzClips['airplane_pair']='飞机带对'
$ddzClips['four_two']='四带二'
$ddzClips['four_pairs']='四带两对'
$ddzClips['bomb']='炸弹'
$ddzClips['rocket']='王炸'
$ddzClips['bid-1']='一分'
$ddzClips['bid-2']='两分'
$ddzClips['bid-3']='三分'
$ddzClips['no-bid']='不叫'
$ddzClips['pass']='不出'
$ddzClips['landlord']='我是地主'
$ddzClips['one-left']='只剩一张牌了'
$ddzClips['two-left']='只剩两张牌了'
$ddzClips['spring']='春天'
$ddzClips['anti-spring']='反春天'
$ddzClips['landlord-win']='地主获胜'
$ddzClips['farmers-win']='农民获胜'
$ddzClips['deal']='开始发牌'
$ddzClips['your-turn']='轮到你出牌'
try {
    foreach($key in $ddzClips.Keys) {
        $wav=Join-Path $ddzScratch "$key.wav"
        $ddzSpeaker.SetOutputToWaveFile($wav)
        $ddzSpeaker.Speak($ddzClips[$key])
        $ddzSpeaker.SetOutputToNull()
        & ffmpeg -hide_banner -loglevel error -y -i $wav -af 'silenceremove=start_periods=1:start_threshold=-45dB:stop_periods=-1:stop_duration=0.12:stop_threshold=-45dB,afade=t=in:d=0.01,loudnorm=I=-18:TP=-2:LRA=7' -ar 24000 -ac 1 -codec:a libmp3lame -b:a 64k (Join-Path $ddzOutput "$key.mp3")
        if($LASTEXITCODE -ne 0){throw "Voice encoding failed: $key"}
    }
} finally { $ddzSpeaker.Dispose() }
$ddzClips | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $ddzOutput 'manifest.json') -Encoding UTF8
Write-Output "Generated $($ddzClips.Count) offline landlord voice clips."

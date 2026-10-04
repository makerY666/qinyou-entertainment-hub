"""Original 108 BPM, 32-bar stereo mahjong music. No external samples.
Two themes, plucked lead, flute answers, chords, bass, drums and fills.
Requires NumPy. All tails wrap around the 71.111-second loop boundary.
"""
import sys
import wave
from pathlib import Path
import numpy as np

RATE = 32000
BEAT = 60 / 108
LENGTH = round(RATE * BEAT * 128)
mix = np.zeros((LENGTH, 2), dtype=np.float64)
rng = np.random.default_rng(10827)

def frequency(midi):
    return 440 * 2 ** ((midi - 69) / 12)

def add(signal, at, gain, pan=0):
    indices = (round(at * BEAT * RATE) + np.arange(len(signal))) % LENGTH
    angle = (pan + 1) * np.pi / 4
    mix[indices, 0] += signal * gain * np.cos(angle)
    mix[indices, 1] += signal * gain * np.sin(angle)

def pluck(note, at, gain=.13, pan=0, length=1.4):
    t = np.arange(round(length * RATE)) / RATE
    phase = 2 * np.pi * frequency(note) * (t + .00005 * (1 - np.exp(-t * 50)))
    signal = np.zeros_like(t)
    for harmonic, weight in [(1, 1), (2, .36), (3, .17), (4, .07), (6, .025)]:
        signal += weight * np.sin(phase * harmonic) * np.exp(-t * (2.5 + harmonic * .8))
    add(signal * (1 - np.exp(-t * 500)), at, gain, pan)

def bass(note, at, gain=.16):
    t = np.arange(round(.7 * BEAT * RATE)) / RATE
    phase = 2 * np.pi * frequency(note) * t
    envelope = (1 - np.exp(-t * 100)) * np.exp(-t * 3) * np.minimum(1, (t[-1] - t) / .035)
    add((np.sin(phase) + .38 * np.sin(phase * 2) + .10 * np.sin(phase * 3)) * envelope, at, gain)

def flute(note, at, beats=1, gain=.052):
    t = np.arange(round(beats * BEAT * RATE)) / RATE
    phase = 2 * np.pi * frequency(note) * t + .10 * np.sin(2 * np.pi * 5.2 * t)
    envelope = np.minimum(1, t / .045) * np.minimum(1, (t[-1] - t) / .12)
    add((np.sin(phase) + .16 * np.sin(2 * phase)) * envelope, at, gain, .25)

def chord(notes, at):
    t = np.arange(round(3.8 * BEAT * RATE)) / RATE
    envelope = (1 - np.exp(-t * 8)) * np.minimum(1, (t[-1] - t) / .25)
    for i, note in enumerate(notes):
        f = frequency(note)
        add((np.sin(2 * np.pi * f * t) + .18 * np.sin(4 * np.pi * f * t)) * envelope,
            at, .015, -.45 + i * .3)

def drum(kind, at, gain=1):
    if kind == 'kick':
        t = np.arange(round(.24 * RATE)) / RATE
        phase = 2 * np.pi * (49 * t + 46 / 32 * (1 - np.exp(-32 * t)))
        signal = np.sin(phase) * np.exp(-t * 19) * (1 - np.exp(-t * 600))
        add(signal, at, .24 * gain)
    elif kind == 'rim':
        t = np.arange(round(.13 * RATE)) / RATE
        signal = (np.sin(2 * np.pi * 920 * t) + .48 * np.sin(2 * np.pi * 1430 * t)) * np.exp(-t * 65)
        noise = rng.normal(size=len(t))
        signal += .12 * (noise - np.roll(noise, 1)) * np.exp(-t * 48)
        add(signal, at, .051 * gain, -.18)
    else:
        t = np.arange(round(.065 * RATE)) / RATE
        noise = rng.normal(size=len(t))
        signal = (noise - np.roll(noise, 1)) * np.exp(-t * 75) * (1 - np.exp(-t * 1000))
        add(signal, at, .016 * gain, .38)

progression = [([50, 54, 57, 62], 38), ([47, 50, 54, 59], 35),
               ([43, 47, 50, 55], 31), ([45, 49, 52, 57], 33)]
theme_a = [
    [(0, 62), (.75, 66), (1.5, 69), (2.5, 66), (3, 64)],
    [(0, 62), (1, 59), (2, 62), (2.75, 66), (3.5, 69)],
    [(0, 67), (.5, 69), (1.5, 71), (2.5, 69), (3.25, 67)],
    [(0, 66), (1, 64), (2.5, 61), (3.5, 64)],
    [(0, 66), (.75, 69), (1.5, 74), (2.5, 69), (3.25, 66)],
    [(0, 71), (1, 69), (1.75, 66), (2.5, 62), (3.5, 66)],
    [(0, 67), (.75, 71), (1.5, 69), (2.5, 67), (3, 62)],
    [(0, 64), (.5, 66), (1.5, 69), (2.5, 64)],
]
theme_b = [
    [(0, 74), (.5, 69), (1.25, 66), (2, 69), (3, 74)],
    [(0, 71), (.75, 74), (1.5, 78), (2.5, 74), (3.25, 71)],
    [(0, 74), (1, 71), (1.75, 67), (2.5, 69), (3.5, 71)],
    [(0, 73), (.5, 71), (1.5, 69), (2.5, 64), (3.25, 69)],
    [(0, 74), (.75, 78), (1.5, 76), (2.5, 74), (3.5, 69)],
    [(0, 71), (.5, 69), (1.5, 66), (2.25, 69), (3, 71)],
    [(0, 67), (.75, 71), (1.5, 74), (2.5, 71), (3.25, 67)],
    [(0, 69), (.75, 66), (1.5, 64), (2.5, 61), (3.5, 64)],
]
for bar in range(32):
    start = bar * 4
    notes, root = progression[bar % 4]
    section = bar // 8
    theme = theme_b if section % 2 else theme_a
    energy = .82 if section == 2 else 1
    chord(notes, start)
    for offset, midi in theme[bar % 8]:
        pluck(midi, start + offset, .13 * energy, -.12)
    for j in range(8):
        pluck(notes[[0, 2, 1, 3, 0, 2, 1, 2][j]] + 12, start + j * .5,
              .027 if section == 2 else .039, .42, .65)
        drum('shaker', start + j * .5, (.6 if j % 2 else 1) * energy)
    for offset, midi in [(0, root), (1.5, root + 7), (2.5, root + 12), (3.5, root + 7)]:
        bass(midi, start + offset, .14 * energy)
    drum('kick', start, energy)
    drum('kick', start + 2, .82 * energy)
    if section != 2 or bar % 2:
        drum('kick', start + 2.75, .48)
    drum('rim', start + 1, energy)
    drum('rim', start + 3, energy)
    if bar % 4 == 3:
        for offset, gain in [(3.25, .38), (3.5, .62), (3.75, .85)]:
            drum('rim', start + offset, gain)
    if bar % 2 == 1:
        flute(notes[2] + 24, start + 2, .7, .036)
        flute(notes[1] + 24, start + 2.75, .95, .032)

dry = mix.copy()
for delay, gain in [(.073, .12), (.137, .08), (.223, .045)]:
    mix += np.roll(dry[:, ::-1], round(delay * RATE), axis=0) * gain
mix = np.tanh(mix * 1.12)
mix *= .79 / max(.79, float(np.abs(mix).max()))
out = Path(sys.argv[1])
out.parent.mkdir(parents=True, exist_ok=True)
with wave.open(str(out), 'wb') as audio:
    audio.setnchannels(2)
    audio.setsampwidth(2)
    audio.setframerate(RATE)
    audio.writeframes((mix * 32767).astype('<i2').tobytes())
print(f'Original 108 BPM / 32 bars / {LENGTH / RATE:.3f}s stereo loop: {out}')

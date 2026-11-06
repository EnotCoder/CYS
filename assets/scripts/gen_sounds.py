#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 EnotCoder
# ========================================================================
#  Генератор 8-битных звуковых эффектов и сезонных музыкальных лупов
#  в стиле текущих sounds/*.ogg.
#  Синтез чистым Python (stdlib) -> WAV -> ffmpeg -> OGG (как у Kenney).
#  Запуск: python3 scripts/gen_sounds.py
# ========================================================================

import math
import os
import random
import struct
import subprocess
import tempfile
import wave

SR = 44100
OUT_DIR = os.path.normpath(os.path.join(os.path.dirname(__file__), "..", "sounds"))


def synth(duration):
    return [0.0] * int(duration * SR)


def add_tone(buf, start, freq, dur, amp, decay=30.0, harmonics=(), fade_in=0.002):
    """Синусоида с гармониками, экспоненциальным спадом и микро-фейдом."""
    n0 = int(start * SR)
    n1 = int((start + dur) * SR)
    divisor = 1.0 + sum(ha for _, ha in harmonics)
    for i in range(n0, min(n1, len(buf))):
        t = (i - n0) / SR
        env = min(1.0, t / fade_in) * math.exp(-decay * t)
        if env < 1e-4:
            continue
        v = math.sin(2 * math.pi * freq * t)
        for h, ha in harmonics:
            v += ha * math.sin(2 * math.pi * freq * h * t)
        buf[i] += amp * env * v / divisor


def add_noise(buf, start, dur, amp, decay=20.0):
    """Короткий белый шум (для ударных/свепа)."""
    n0 = int(start * SR)
    n1 = int((start + dur) * SR)
    for i in range(n0, min(n1, len(buf))):
        t = (i - n0) / SR
        env = math.exp(-decay * t)
        if env < 1e-4:
            continue
        buf[i] += amp * env * (random.random() * 2 - 1)


def add_steady_noise(buf, start, dur, amp, lowpass=0.2, wobble=1.0):
    """Стационарный цветной шум с лёгкой фильтрацией и медленным покачиванием голоса."""
    n0 = int(start * SR)
    n1 = int((start + dur) * SR)
    prev = 0.0
    for i in range(n0, min(n1, len(buf))):
        t = (i - n0) / SR
        prev = prev + lowpass * ((random.random() * 2 - 1) - prev)
        wob = math.sin(2 * math.pi * wobble * t)
        buf[i] += amp * (0.75 + 0.25 * wob) * prev


def add_sweep(buf, start, f0, f1, dur, amp, decay=8.0):
    """Синусоида с плавным скольжением частоты f0 -> f1."""
    n0 = int(start * SR)
    n1 = int((start + dur) * SR)
    for i in range(n0, min(n1, len(buf))):
        t = (i - n0) / SR
        phase_twopi = f0 * t + 0.5 * (f1 - f0) * t * t / dur
        env = math.exp(-decay * t)
        if env < 1e-4:
            continue
        buf[i] += amp * env * math.sin(2 * math.pi * phase_twopi)


def render(name, buf):
    os.makedirs(OUT_DIR, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        wav = os.path.join(tmp, name + ".wav")
        with wave.open(wav, "wb") as w:
            w.setnchannels(1)
            w.setsampwidth(2)
            w.setframerate(SR)
            frames = bytearray()
            for s in buf:
                v = int(max(-1.0, min(1.0, s)) * 32767)
                frames += struct.pack("<h", v)
            w.writeframes(bytes(frames))
        ogg = os.path.join(OUT_DIR, name + ".ogg")
        subprocess.run(
            ["ffmpeg", "-y", "-i", wav, "-c:a", "libvorbis", "-q:a", "5", ogg],
            check=True, capture_output=True,
        )
    print(f"generated {ogg}")


def build_click():
    b = synth(0.09)
    add_tone(b, 0.0, 1000.0, 0.09, 0.6, decay=35.0, harmonics=((2, 0.35), (3, 0.15)))
    return b


def build_hover():
    b = synth(0.06)
    add_tone(b, 0.0, 720.0, 0.06, 0.35, decay=55.0)
    return b


def build_error():
    b = synth(0.22)
    add_tone(b, 0.0, 130.0, 0.22, 0.5, decay=9.0, harmonics=((2, 0.4),))
    add_noise(b, 0.0, 0.15, 0.25, decay=25.0)
    return b


def build_bell():
    b = synth(0.8)
    add_tone(b, 0.0, 660.0, 0.7, 0.45, decay=6.0, harmonics=((2, 0.3), (3, 0.15)))
    add_tone(b, 0.28, 880.0, 0.5, 0.4, decay=7.0, harmonics=((2, 0.3), (3, 0.15)))
    return b


def build_cash():
    b = synth(0.55)
    for i, f in enumerate((1568.0, 1318.5, 1174.7, 1046.5)):
        add_tone(b, i * 0.09, f, 0.35, 0.5, decay=18.0, harmonics=((2, 0.4), (3, 0.2)))
    return b


def build_pickup():
    b = synth(0.13)
    add_noise(b, 0.0, 0.1, 0.45, decay=40.0)
    add_sweep(b, 0.0, 480.0, 260.0, 0.11, 0.4, decay=30.0)
    return b


def build_candy():
    b = synth(0.16)
    add_tone(b, 0.0, 950.0, 0.16, 0.5, decay=20.0, harmonics=((2, 0.3), (4, 0.2)))
    return b


def build_stair():
    b = synth(0.45)
    add_sweep(b, 0.0, 380.0, 120.0, 0.4, 0.5, decay=5.0)
    add_noise(b, 0.0, 0.4, 0.15, decay=10.0)
    return b


def build_save():
    b = synth(0.55)
    for i, f in enumerate((523.25, 659.25, 783.99)):
        add_tone(b, i * 0.15, f, 0.5, 0.4, decay=6.0, harmonics=((2, 0.3), (3, 0.15)))
    return b


def build_rain():
    b = synth(2.4)
    add_steady_noise(b, 0.0, 2.4, 0.5, lowpass=0.35, wobble=6.0)
    return b


def build_snow():
    b = synth(2.4)
    add_steady_noise(b, 0.0, 2.4, 0.28, lowpass=0.06, wobble=1.5)
    return b


def build_thunder():
    b = synth(2.2)
    add_noise(b, 0.0, 0.4, 0.7, decay=8.0)
    add_tone(b, 0.0, 55.0, 2.0, 0.55, decay=2.2, harmonics=((2, 0.5), (3, 0.3)))
    add_tone(b, 0.35, 40.0, 1.6, 0.4, decay=2.5, harmonics=((2, 0.5),))
    return b


def midi_freq(note):
    """Частота ноты MIDI (69 = A4 = 440 Гц)."""
    return 440.0 * (2.0 ** ((note - 69) / 12.0))


def add_pad(buf, start, dur, note, amp):
    """Мягкий аккордовый пад с плавной атакой и затуханием на краях."""
    n0 = int(start * SR)
    n1 = min(int((start + dur) * SR), len(buf))
    total = n1 - n0
    if total <= 0:
        return
    attack = max(1, int(0.35 * SR))
    release = max(1, int(min(0.9, dur / 3.0) * SR))
    f = midi_freq(note)
    for i in range(n0, n1):
        p = i - n0
        t = p / SR
        env = 1.0
        if p < attack:
            env = p / attack
        if p > total - release:
            env = min(env, (total - p) / release)
        v = math.sin(2 * math.pi * f * t)
        v += 0.28 * math.sin(2 * math.pi * 2 * f * t)
        v += 0.10 * math.sin(2 * math.pi * 3 * f * t)
        buf[i] += amp * env * v / 1.38


def build_track(bpm, chords, melody, bass, pad_amp=0.10, lead_amp=0.20,
                lead_decay=2.0, harmonics=((2, 0.30), (3, 0.12)),
                shaker=False, wind=0.0):
    """Собирает зацикленную сезонную тему: пад, бас, мелодия и шум."""
    beat = 60.0 / bpm
    bars = len(chords)
    duration = bars * 4 * beat
    b = synth(duration)

    for bar, chord in enumerate(chords):
        start = bar * 4 * beat
        for note in chord:
            add_pad(b, start, 4 * beat, note, pad_amp)
        root = chord[0] - 12
        for step, on in enumerate(bass):
            if on:
                add_tone(b, start + step * beat, midi_freq(root),
                         beat * 0.85, 0.16, decay=1.6,
                         harmonics=((2, 0.20),))

    for start_beat, note, beats in melody:
        add_tone(b, start_beat * beat, midi_freq(note), beats * beat,
                 lead_amp, decay=lead_decay, harmonics=harmonics)

    if shaker:
        for i in range(int(duration / beat * 2)):
            add_noise(b, i * beat / 2, 0.05, 0.035, decay=55.0)

    if wind > 0.0:
        add_steady_noise(b, 0.0, duration, wind, lowpass=0.05, wobble=0.35)

    # Короткий фейд на стыке лупа убирает щелчок при repeat_infinite().
    fade = int(0.008 * SR)
    for i in range(fade):
        g = i / fade
        b[i] *= g
        b[-1 - i] *= g

    peak = max(abs(x) for x in b) or 1.0
    if peak > 0.8:
        b = [x * (0.8 / peak) for x in b]
    return b


def build_music_spring():
    chords = [
        [50, 57, 62], [45, 52, 57], [47, 54, 59], [43, 50, 55],
        [50, 57, 62], [45, 52, 57], [43, 50, 55], [45, 52, 57],
    ]
    melody = [
        (0, 62, 1), (1, 66, 1), (2, 69, 1), (3, 74, 1.5),
        (5, 73, 1), (6, 71, 1), (7, 69, 1.5),
        (9, 66, 1), (10, 69, 1), (11, 74, 1.5),
        (13, 71, 1), (14, 69, 1), (15, 66, 1.5),
        (16, 69, 1), (17, 74, 1), (18, 78, 1.5), (20, 76, 1), (21, 74, 1), (22, 69, 1.5),
        (24, 66, 1), (25, 69, 1), (26, 71, 1.5), (28, 69, 1), (29, 66, 1), (30, 62, 1.5),
    ]
    return build_track(96, chords, melody, [1, 0, 0, 1], pad_amp=0.11,
                       lead_amp=0.22, lead_decay=2.4,
                       harmonics=((2, 0.35), (3, 0.18)))


def build_music_summer():
    chords = [
        [48, 55, 60], [43, 50, 55], [45, 52, 57], [41, 48, 53],
        [48, 55, 60], [43, 50, 55], [41, 48, 53], [43, 50, 55],
    ]
    melody = [
        (0, 72, 1), (1, 76, 1), (2, 79, 1), (3, 81, 1.5),
        (5, 79, 1), (6, 76, 1), (7, 72, 1.5),
        (8, 74, 1), (9, 77, 1), (10, 81, 1.5), (12, 79, 1), (13, 77, 1), (14, 74, 1.5),
        (16, 72, 1), (17, 76, 1), (18, 79, 1.5), (20, 84, 1), (21, 81, 1), (22, 79, 1.5),
        (24, 77, 1), (25, 74, 1), (26, 72, 1.5), (28, 74, 1), (29, 77, 1), (30, 79, 1.5),
    ]
    return build_track(120, chords, melody, [1, 0, 1, 0], pad_amp=0.10,
                       lead_amp=0.20, lead_decay=2.6,
                       harmonics=((2, 0.40), (3, 0.20)), shaker=True)


def build_music_autumn():
    chords = [
        [45, 52, 57], [41, 48, 53], [48, 55, 60], [43, 50, 55],
        [45, 52, 57], [41, 48, 53], [43, 50, 55], [45, 52, 57],
    ]
    melody = [
        (0, 69, 1.5), (2, 72, 1), (3, 71, 1.5),
        (5, 69, 1), (6, 67, 1), (7, 64, 1.5),
        (9, 67, 1), (10, 69, 1), (11, 72, 1.5),
        (13, 71, 1), (14, 67, 1), (15, 64, 1.5),
        (16, 69, 1), (17, 72, 1), (18, 74, 1.5), (20, 72, 1), (21, 69, 1), (22, 67, 1.5),
        (24, 64, 1), (25, 67, 1), (26, 69, 1.5), (28, 67, 1), (29, 64, 1), (30, 62, 1.5),
    ]
    return build_track(84, chords, melody, [1, 0, 0, 0], pad_amp=0.13,
                       lead_amp=0.19, lead_decay=1.8,
                       harmonics=((2, 0.22), (3, 0.10)), wind=0.025)


def build_music_winter():
    chords = [
        [40, 47, 52], [36, 43, 48], [43, 50, 55], [38, 45, 50],
        [40, 47, 52], [36, 43, 48], [38, 45, 50], [40, 47, 52],
    ]
    melody = [
        (0, 76, 2), (2, 79, 2), (4, 83, 2), (6, 81, 2),
        (8, 79, 2), (10, 76, 2), (12, 74, 2), (14, 71, 2),
        (16, 76, 1.5), (18, 79, 1.5), (20, 84, 2), (22, 83, 1.5), (24, 81, 2), (26, 79, 1.5),
        (28, 76, 1.5), (30, 74, 2),
    ]
    return build_track(70, chords, melody, [1, 0, 0, 0], pad_amp=0.14,
                       lead_amp=0.15, lead_decay=1.3,
                       harmonics=((2, 0.45), (3, 0.22), (5, 0.08)), wind=0.018)


def build_buy():
    b = synth(0.35)
    add_tone(b, 0.0, 784.0, 0.22, 0.4, decay=12.0, harmonics=((2, 0.3), (3, 0.15)))
    add_tone(b, 0.07, 1046.5, 0.26, 0.4, decay=10.0, harmonics=((2, 0.3), (3, 0.15)))
    return b


def build_open():
    b = synth(0.16)
    add_sweep(b, 0.0, 260.0, 880.0, 0.16, 0.35, decay=18.0)
    return b


def build_close():
    b = synth(0.16)
    add_sweep(b, 0.0, 880.0, 260.0, 0.16, 0.35, decay=18.0)
    return b


def build_step():
    b = synth(0.06)
    add_noise(b, 0.0, 0.06, 0.3, decay=45.0)
    return b


def build_swish():
    b = synth(0.2)
    add_sweep(b, 0.0, 200.0, 1400.0, 0.18, 0.25, decay=14.0)
    return b


def main():
    for name, fn in {
        "click": build_click,
        "hover": build_hover,
        "error": build_error,
        "bell": build_bell,
        "cash": build_cash,
        "pickup": build_pickup,
        "candy": build_candy,
        "stair": build_stair,
        "save": build_save,
        "rain": build_rain,
        "snow": build_snow,
        "thunder": build_thunder,
        "buy": build_buy,
        "open": build_open,
        "close": build_close,
        "step": build_step,
        "swish": build_swish,
        "music_spring": build_music_spring,
        "music_summer": build_music_summer,
        "music_autumn": build_music_autumn,
        "music_winter": build_music_winter,
    }.items():
        render(name, fn())


if __name__ == "__main__":
    main()
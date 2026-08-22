#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 EnotCoder
# ========================================================================
#  Генератор 8-битных звуковых эффектов в стиле текущих sounds/*.ogg
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
    }.items():
        render(name, fn())


if __name__ == "__main__":
    main()
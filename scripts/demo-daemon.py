#!/usr/bin/env python3
"""Faux daemon VeeMeeter pour les captures d'écran du README.

Sert un état de démonstration et des VU-mètres animés sur le même protocole
que le vrai daemon (une ligne JSON par message). Aucun accès à PipeWire.

    XDG_RUNTIME_DIR=/tmp/vm-demo python3 scripts/demo-daemon.py &
    XDG_RUNTIME_DIR=/tmp/vm-demo veemeeter gui --no-spawn
"""
import json, math, os, socket, threading, time

def strip(id, kind, name, routes=(), device=None, ch=("FL", "FR"), db=0.0, mute=False, default=False):
    s = {"id": id, "kind": kind, "name": name, "channels": list(ch), "volume_db": db, "mute": mute,
         "ready": True, "online": True, "is_default": default}
    if routes: s["routes"] = list(routes)
    if device: s["device"] = device
    if default: s["primary"] = True
    return s

STATE = {
    "strips": [
        strip("hi1", "hardware_in", "Microphone", ["b1"], "alsa_input.usb-mic", ["MONO"], 2.5),
        strip("vi1", "virtual_in", "Desktop", ["a1"], default=True),
        strip("vi2", "virtual_in", "Music", ["a1", "b1"], db=-9.0),
        strip("vi3", "virtual_in", "Voice chat", ["a1"], db=-3.5),
        strip("a1", "hardware_out", "Headphones", device="alsa_output.analog"),
        strip("a2", "hardware_out", "Stream PC", device="alsa_output.hdmi", db=-6.0, mute=True),
        strip("b1", "virtual_out", "Virtual Mic", default=True),
    ],
    "devices": [
        {"name": "alsa_output.analog", "description": "Built-in Headphones", "capture": False, "channels": ["FL", "FR"]},
        {"name": "alsa_output.hdmi", "description": "HDMI / DisplayPort", "capture": False, "channels": ["FL", "FR"]},
        {"name": "alsa_input.usb-mic", "description": "USB Microphone", "capture": True, "channels": ["MONO"]},
    ],
    "apps": [
        {"id": 71, "name": "Firefox", "media": "Lo-fi beats to mix to", "icon": None, "playback": True,
         "target": "veemeeter.vi1", "target_strip": "vi1"},
        {"id": 72, "name": "Spotify", "media": "Midnight Pastels — Sakura Drive", "icon": None, "playback": True,
         "target": "veemeeter.vi2", "target_strip": "vi2"},
        {"id": 73, "name": "Discord", "media": "Voice call", "icon": None, "playback": True,
         "target": "veemeeter.vi3", "target_strip": "vi3"},
        {"id": 74, "name": "Discord", "media": "Microphone", "icon": None, "playback": False,
         "target": "veemeeter.b1", "target_strip": "b1"},
        {"id": 75, "name": "OBS Studio", "media": "Mic/Aux", "icon": None, "playback": False,
         "target": "veemeeter.b1", "target_strip": "b1"},
    ],
    "default_sink": "veemeeter.vi1",
    "default_source": "veemeeter.b1",
    "pipewire_version": "1.6.11",
}

# Niveaux crête de base (amplitude) par strip, modulés dans le temps.
BASE = {"hi1": [0.42], "vi1": [0.20, 0.18], "vi2": [0.80, 0.74], "vi3": [0.33, 0.33],
        "a1": [0.62, 0.58], "a2": [0.0, 0.0], "b1": [0.55, 0.52]}

def serve(conn):
    f = conn.makefile("r")
    def send(msg):
        conn.sendall((json.dumps(msg) + "\n").encode())
    try:
        f.readline()  # subscribe
        send({"msg": "state", "state": STATE})
        t0 = time.time()
        while True:
            t = time.time() - t0
            levels = [[k, [v * (0.75 + 0.25 * math.sin(3.1 * t + i + len(k))) for i, v in enumerate(vs)]]
                      for k, vs in BASE.items()]
            send({"msg": "meters", "levels": levels})
            time.sleep(1 / 30)
    except OSError:
        pass

def main():
    path = os.path.join(os.environ["XDG_RUNTIME_DIR"], "veemeeter.sock")
    try:
        os.unlink(path)
    except FileNotFoundError:
        pass
    srv = socket.socket(socket.AF_UNIX)
    srv.bind(path)
    srv.listen()
    while True:
        conn, _ = srv.accept()
        threading.Thread(target=serve, args=(conn,), daemon=True).start()

if __name__ == "__main__":
    main()

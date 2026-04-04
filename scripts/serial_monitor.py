#!/usr/bin/env python3
"""Serial monitor for ESP32-C3 USB JTAG.

Usage:
    python3 scripts/serial_monitor.py [port]

Runs as a daemon. Logs all serial output to /tmp/esp32-serial.log.
Automatically reconnects. Releases the port when the lock file
/tmp/esp32-serial.lock is removed (e.g. for flashing), and reclaims
it when the lock file reappears.

Control:
    Start:   python3 scripts/serial_monitor.py
    Stop:    kill $(cat /tmp/esp32-serial.pid)
    Pause:   rm /tmp/esp32-serial.lock   (releases port for flashing)
    Resume:  touch /tmp/esp32-serial.lock (reclaims port)
"""

import os
import sys
import time

import serial

LOG_PATH = "/tmp/esp32-serial.log"
PID_PATH = "/tmp/esp32-serial.pid"
LOCK_PATH = "/tmp/esp32-serial.lock"

port = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM1"

# Write PID
with open(PID_PATH, "w") as f:
    f.write(str(os.getpid()))

# Create lock file (presence = monitor owns the port)
open(LOCK_PATH, "w").close()

logfile = open(LOG_PATH, "a")


def log(msg):
    logfile.write(msg + "\n")
    logfile.flush()


log(f"[monitor started, port={port}, pid={os.getpid()}]")

ser = None

try:
    while True:
        # If lock file removed, release port and wait
        if not os.path.exists(LOCK_PATH):
            if ser is not None:
                try:
                    ser.close()
                except Exception:
                    pass
                ser = None
                log("[port released for flashing]")
            time.sleep(0.5)
            continue

        # Try to connect if not connected
        if ser is None:
            try:
                ser = serial.Serial(port, 115200, timeout=1)
                log(f"[connected to {port}]")
            except serial.SerialException:
                time.sleep(1)
                continue

        # Read a line
        try:
            line = ser.readline()
            if line:
                text = line.decode("utf-8", errors="replace").rstrip()
                log(text)
        except serial.SerialException:
            try:
                ser.close()
            except Exception:
                pass
            ser = None
            log("[disconnected]")
            time.sleep(1)

except KeyboardInterrupt:
    pass
finally:
    if ser:
        try:
            ser.close()
        except Exception:
            pass
    logfile.close()
    try:
        os.remove(PID_PATH)
    except Exception:
        pass
    try:
        os.remove(LOCK_PATH)
    except Exception:
        pass

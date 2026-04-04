#!/usr/bin/env python3
"""Serial monitor for ESP32-C3 USB JTAG.

Usage:
    python3 scripts/serial_monitor.py [seconds] [port]

Reads line-buffered serial output for the given duration (default: 10s).
Prints clean lines to stdout. Intended for non-interactive use (e.g. Claude Code).
"""

import sys
import time
import serial

duration = int(sys.argv[1]) if len(sys.argv) > 1 else 10
port = sys.argv[2] if len(sys.argv) > 2 else "/dev/ttyACM1"

try:
    ser = serial.Serial(port, 115200, timeout=1)
except serial.SerialException as e:
    print(f"Error opening {port}: {e}", file=sys.stderr)
    sys.exit(1)

end = time.time() + duration
while time.time() < end:
    line = ser.readline()
    if line:
        try:
            print(line.decode("utf-8", errors="replace").rstrip())
        except Exception:
            pass

ser.close()

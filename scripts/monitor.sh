#!/bin/bash
# Watch live serial output from the ESP32.
# The serial_monitor.py daemon must be running.
# Usage: ./scripts/monitor.sh
tail -f /tmp/esp32-serial.log

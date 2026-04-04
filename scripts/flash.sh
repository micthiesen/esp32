#!/bin/bash
# Flash firmware to ESP32-C3, pausing the serial monitor if running.
# Usage: bash scripts/flash.sh <binary-path> [port]

BINARY="${1:?Usage: flash.sh <binary-path> [port]}"
PORT="${2:-/dev/ttyACM1}"
LOCK="/tmp/esp32-serial.lock"

# Pause monitor
if [ -f "$LOCK" ]; then
    rm "$LOCK"
    sleep 1
fi

# Flash
espflash flash --port "$PORT" "$BINARY"
EXIT=$?

# Resume monitor
touch "$LOCK"

exit $EXIT

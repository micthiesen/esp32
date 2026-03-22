# Wiring Checklist

Soldering order for protoboard build. Check off each connection as you make it.
Test after each phase before moving on.

## Phase 1: Power Rails

Set up 5V and GND rails on the protoboard. ESP32 3.3V LDO provides the 3.3V rail.

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | USB charger | 5V | | 5V rail | protoboard | |
| | USB charger | GND | | GND rail | protoboard | |
| | protoboard | 5V rail | | 5V | ESP32 | |
| | protoboard | GND rail | | GND | ESP32 | |
| | ESP32 | 3.3V | | 3.3V rail | protoboard | LDO output |

**Test:** Power on. ESP32 should boot (check serial output).

## Phase 2: I2C Bus + ADC Modules

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | protoboard | 3.3V rail | | VDD | ADC1 | |
| | protoboard | GND rail | | GND | ADC1 | |
| | protoboard | GND rail | | ADDR | ADC1 | sets address 0x48 |
| | protoboard | 3.3V rail | | VDD | ADC2 | |
| | protoboard | GND rail | | GND | ADC2 | |
| | protoboard | 3.3V rail | | ADDR | ADC2 | sets address 0x49 |
| | ESP32 | GPIO0 | | SDA | ADC1 | |
| | ESP32 | GPIO0 | | SDA | ADC2 | same node as above |
| | ESP32 | GPIO1 | | SCL | ADC1 | |
| | ESP32 | GPIO1 | | SCL | ADC2 | same node as above |
| | protoboard | 3.3V rail | 4.7kΩ | SDA line | protoboard | I2C pull-up |
| | protoboard | 3.3V rail | 4.7kΩ | SCL line | protoboard | I2C pull-up |

**Test:** Flash firmware, check serial for I2C scan finding 0x48 and 0x49.

## Phase 3: Discharge Circuits

Each channel has 6 connections. Do one channel at a time and test it before moving to the next.

IRLZ44N pinout (flat side facing you, legs down): **Gate | Drain | Source**

### Slot 1 — AA

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO2 | | gate | Q1 (IRLZ44N) | |
| | Q1 (IRLZ44N) | source | | GND rail | protoboard | |
| | Batt 1 holder | + | | leg A | R1 (2.2Ω 5W) | |
| | R1 (2.2Ω 5W) | leg B | | drain | Q1 (IRLZ44N) | |
| | Batt 1 holder | + | | A0 | ADC1 | tee off same node as R1 leg A |
| | Batt 1 holder | - | | GND rail | protoboard | |

### Slot 2 — AA

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO3 | | gate | Q2 (IRLZ44N) | |
| | Q2 (IRLZ44N) | source | | GND rail | protoboard | |
| | Batt 2 holder | + | | leg A | R2 (2.2Ω 5W) | |
| | R2 (2.2Ω 5W) | leg B | | drain | Q2 (IRLZ44N) | |
| | Batt 2 holder | + | | A1 | ADC1 | tee off same node as R2 leg A |
| | Batt 2 holder | - | | GND rail | protoboard | |

### Slot 3 — AA

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO4 | | gate | Q3 (IRLZ44N) | |
| | Q3 (IRLZ44N) | source | | GND rail | protoboard | |
| | Batt 3 holder | + | | leg A | R3 (2.2Ω 5W) | |
| | R3 (2.2Ω 5W) | leg B | | drain | Q3 (IRLZ44N) | |
| | Batt 3 holder | + | | A2 | ADC1 | tee off same node as R3 leg A |
| | Batt 3 holder | - | | GND rail | protoboard | |

### Slot 4 — AA

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO5 | | gate | Q4 (IRLZ44N) | |
| | Q4 (IRLZ44N) | source | | GND rail | protoboard | |
| | Batt 4 holder | + | | leg A | R4 (2.2Ω 5W) | |
| | R4 (2.2Ω 5W) | leg B | | drain | Q4 (IRLZ44N) | |
| | Batt 4 holder | + | | A3 | ADC1 | tee off same node as R4 leg A |
| | Batt 4 holder | - | | GND rail | protoboard | |

### Slot 5 — AAA

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO6 | | gate | Q5 (IRLZ44N) | |
| | Q5 (IRLZ44N) | source | | GND rail | protoboard | |
| | Batt 5 holder | + | | leg A | R5 (2.2Ω 5W) | |
| | R5 (2.2Ω 5W) | leg B | | drain | Q5 (IRLZ44N) | |
| | Batt 5 holder | + | | A0 | ADC2 | tee off same node as R5 leg A |
| | Batt 5 holder | - | | GND rail | protoboard | |

### Slot 6 — AAA

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO7 | | gate | Q6 (IRLZ44N) | |
| | Q6 (IRLZ44N) | source | | GND rail | protoboard | |
| | Batt 6 holder | + | | leg A | R6 (2.2Ω 5W) | |
| | R6 (2.2Ω 5W) | leg B | | drain | Q6 (IRLZ44N) | |
| | Batt 6 holder | + | | A1 | ADC2 | tee off same node as R6 leg A |
| | Batt 6 holder | - | | GND rail | protoboard | |

### Slot 7 — AAA

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO8 | | gate | Q7 (IRLZ44N) | |
| | Q7 (IRLZ44N) | source | | GND rail | protoboard | |
| | Batt 7 holder | + | | leg A | R7 (2.2Ω 5W) | |
| | R7 (2.2Ω 5W) | leg B | | drain | Q7 (IRLZ44N) | |
| | Batt 7 holder | + | | A2 | ADC2 | tee off same node as R7 leg A |
| | Batt 7 holder | - | | GND rail | protoboard | |

### Slot 8 — AAA

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO9 | | gate | Q8 (IRLZ44N) | |
| | Q8 (IRLZ44N) | source | | GND rail | protoboard | |
| | Batt 8 holder | + | | leg A | R8 (2.2Ω 5W) | |
| | R8 (2.2Ω 5W) | leg B | | drain | Q8 (IRLZ44N) | |
| | Batt 8 holder | + | | A3 | ADC2 | tee off same node as R8 leg A |
| | Batt 8 holder | - | | GND rail | protoboard | |

**Test per slot:** Insert a battery, run `STATUS`. Should read ~1.2-1.4V on that slot, ~0V on others.

## Phase 4: LED Strip

| ✓ | Device | Pin | Wire | Pin | Device | Note |
|---|--------|-----|------|-----|--------|------|
| | ESP32 | GPIO10 | | DIN | SK6812 strip | data line |
| | protoboard | 5V rail | | VDD | SK6812 strip | |
| | protoboard | GND rail | | GND | SK6812 strip | |

**Test:** Insert a battery. LED for that slot should light up (yellow = scanning, then green/red).

## Reminders

- **2.2Ω resistors** are 5W rated (physically large ceramic). They get warm during discharge, that's normal.
- **IRLZ44N pinout** (flat side facing you, legs down): Gate | Drain | Source. Metal tab = Drain.
- **"Tee off same node"** means the voltage sense wire and resistor leg A connect to the same pad/trace as the battery positive wire. It's one shared connection point with 3 things meeting.
- **Battery holders** connect off-board via wires to the protoboard.
- **If LEDs flicker**, add a 3.3V→5V level shifter on the GPIO10 data line.

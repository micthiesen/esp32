// ADS1115 custom chip for Wokwi simulator.
//
// Implements the I2C slave protocol that the ads1x1x Rust crate expects.
// Simulates battery voltage decay when LOAD pins go HIGH.

#include "wokwi-api.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// ADS1115 registers
#define REG_CONVERSION 0x00
#define REG_CONFIG     0x01
#define REG_LO_THRESH  0x02
#define REG_HI_THRESH  0x03

// Config register bits
#define CONFIG_OS_BIT    15
#define CONFIG_MUX_SHIFT 12
#define CONFIG_MUX_MASK  0x7

typedef struct {
  // I2C state
  uint8_t  pointer_reg;
  uint16_t config_reg;
  uint8_t  selected_mux;

  // Write buffer for multi-byte I2C writes
  uint8_t  write_buf[3];
  uint8_t  write_pos;
  bool     is_read;

  // Read state
  uint16_t read_value;
  uint8_t  read_byte;

  // Voltage simulation
  float    initial_voltage[4];
  float    frozen_voltage[4];
  float    decay_rate;         // V/s (converted from mV/s attr)
  bool     load_active[4];
  uint64_t load_start_ns[4];

  // LOAD pin references
  pin_t    load_pins[4];
} chip_state_t;

static chip_state_t *global_state = NULL;

static float get_voltage(chip_state_t *state, uint8_t channel) {
  if (channel >= 4) return 0.0f;

  float voltage = state->frozen_voltage[channel];

  if (state->load_active[channel]) {
    uint64_t now = get_sim_nanos();
    float elapsed_s = (float)(now - state->load_start_ns[channel]) / 1e9f;
    float decay = state->decay_rate * elapsed_s;
    voltage = state->frozen_voltage[channel] - decay;
    if (voltage < 0.0f) voltage = 0.0f;
  }

  return voltage;
}

static int16_t voltage_to_raw(float voltage) {
  // PGA +/-4.096V: 1 LSB = 0.125 mV = 0.000125 V
  return (int16_t)(voltage / 0.000125f);
}

// MUX values 4-7 = single-ended AIN0-AIN3
static uint8_t mux_to_channel(uint8_t mux) {
  if (mux >= 4 && mux <= 7) return mux - 4;
  return 0;
}

static void process_write(chip_state_t *state) {
  if (state->write_pos == 0) return;

  // First byte is always the pointer register
  state->pointer_reg = state->write_buf[0] & 0x03;

  // 3 bytes = pointer + 2 data bytes (register write)
  if (state->write_pos == 3 && state->pointer_reg == REG_CONFIG) {
    uint16_t value = ((uint16_t)state->write_buf[1] << 8) | state->write_buf[2];
    state->config_reg = value;
    state->selected_mux = (value >> CONFIG_MUX_SHIFT) & CONFIG_MUX_MASK;
  }
}

static uint16_t get_register_value(chip_state_t *state) {
  switch (state->pointer_reg) {
    case REG_CONVERSION: {
      uint8_t channel = mux_to_channel(state->selected_mux);
      float voltage = get_voltage(state, channel);
      return (uint16_t)voltage_to_raw(voltage);
    }
    case REG_CONFIG:
      // Always return with OS=1 (conversion complete)
      return state->config_reg | (1u << CONFIG_OS_BIT);
    case REG_LO_THRESH:
      return 0x8000;
    case REG_HI_THRESH:
      return 0x7FFF;
    default:
      return 0;
  }
}

// I2C callbacks
static bool on_connect(void *user_data, uint32_t address, bool read) {
  chip_state_t *state = (chip_state_t *)user_data;
  (void)address;
  state->is_read = read;
  state->write_pos = 0;
  if (read) {
    // Prepare the value to read based on current pointer register
    state->read_value = get_register_value(state);
    state->read_byte = 0;
  }
  return true; // ACK
}

static uint8_t on_read(void *user_data) {
  chip_state_t *state = (chip_state_t *)user_data;
  uint8_t byte;
  if (state->read_byte == 0) {
    byte = (state->read_value >> 8) & 0xFF;  // MSB first
  } else {
    byte = state->read_value & 0xFF;
  }
  state->read_byte++;
  return byte;
}

static bool on_write(void *user_data, uint8_t data) {
  chip_state_t *state = (chip_state_t *)user_data;
  if (state->write_pos < sizeof(state->write_buf)) {
    state->write_buf[state->write_pos++] = data;
  }
  // First byte sets the pointer register immediately
  // (needed for write-then-read sequences without a STOP in between)
  if (state->write_pos == 1) {
    state->pointer_reg = data & 0x03;
  }
  return true; // ACK
}

static void on_disconnect(void *user_data) {
  chip_state_t *state = (chip_state_t *)user_data;
  if (!state->is_read) {
    process_write(state);
  }
}

// GPIO callback for LOAD pins
static void on_load_pin_change(void *user_data, pin_t pin, uint32_t value) {
  chip_state_t *state = (chip_state_t *)user_data;

  for (int i = 0; i < 4; i++) {
    if (state->load_pins[i] == pin) {
      if (value && !state->load_active[i]) {
        // Load activated: freeze current voltage, start decaying from it
        state->frozen_voltage[i] = get_voltage(state, i);
        state->load_active[i] = true;
        state->load_start_ns[i] = get_sim_nanos();
        printf("[ADS1115] CH%d load ON, voltage=%.3fV\n", i, state->frozen_voltage[i]);
      } else if (!value && state->load_active[i]) {
        // Load deactivated: freeze voltage
        state->frozen_voltage[i] = get_voltage(state, i);
        state->load_active[i] = false;
        printf("[ADS1115] CH%d load OFF, voltage=%.3fV\n", i, state->frozen_voltage[i]);
      }
      break;
    }
  }
}

void chip_init(void) {
  chip_state_t *state = malloc(sizeof(chip_state_t));
  memset(state, 0, sizeof(chip_state_t));
  global_state = state;

  // Read configuration attributes (attr_init returns an ID, attr_read uses it)
  uint32_t addr_attr = attr_init("i2c_address", 0x48);
  uint32_t i2c_address = attr_read(addr_attr);

  uint32_t decay_attr = attr_init_float("decay_mv_per_sec", 6.0f);
  state->decay_rate = attr_read_float(decay_attr) / 1000.0f;  // Convert mV/s to V/s

  // Read per-channel initial voltages
  const char *voltage_names[] = {"voltage_a0", "voltage_a1", "voltage_a2", "voltage_a3"};
  float voltage_defaults[] = {1.35f, 1.25f, 0.0f, 1.55f};
  for (int i = 0; i < 4; i++) {
    uint32_t v_attr = attr_init_float(voltage_names[i], voltage_defaults[i]);
    state->initial_voltage[i] = attr_read_float(v_attr);
    state->frozen_voltage[i] = state->initial_voltage[i];
  }

  // Default config register
  state->config_reg = 0x8583;
  state->selected_mux = 4; // AIN0 single-ended

  // Initialize I2C slave
  i2c_config_t i2c_config = {
    .address = i2c_address,
    .sda = pin_init("SDA", INPUT_PULLUP),
    .scl = pin_init("SCL", INPUT_PULLUP),
    .connect = on_connect,
    .read = on_read,
    .write = on_write,
    .disconnect = on_disconnect,
    .user_data = state,
  };
  memset(i2c_config.reserved, 0, sizeof(i2c_config.reserved));
  i2c_init(&i2c_config);

  // Initialize LOAD pins with edge detection
  const char *load_names[] = {"LOAD0", "LOAD1", "LOAD2", "LOAD3"};
  for (int i = 0; i < 4; i++) {
    state->load_pins[i] = pin_init(load_names[i], INPUT);
    pin_watch_config_t watch = {
      .user_data = state,
      .edge = BOTH,
      .pin_change = on_load_pin_change,
    };
    pin_watch(state->load_pins[i], &watch);
  }

  printf("[ADS1115 @0x%02X] Init: A0=%.3fV A1=%.3fV A2=%.3fV A3=%.3fV decay=%.1f mV/s\n",
    i2c_address,
    state->initial_voltage[0], state->initial_voltage[1],
    state->initial_voltage[2], state->initial_voltage[3],
    state->decay_rate * 1000.0f);
}

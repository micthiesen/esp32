#ifndef HELPERS_H
#define HELPERS_H

#include <stdbool.h>
#include <stdint.h>

// Timing helpers
void delay_ms(uint32_t ms);
uint32_t get_time_ms(void);

// LED helpers
void led_init(int gpio_num);
void led_on(int gpio_num);
void led_off(int gpio_num);
void led_toggle(int gpio_num);

// Debug helpers
void hex_dump(const char *desc, const void *addr, int len);

// Math helpers
int map_range(int value, int in_min, int in_max, int out_min, int out_max);
int constrain(int value, int min, int max);

#endif // HELPERS_H

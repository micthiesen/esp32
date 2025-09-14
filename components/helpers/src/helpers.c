#include "helpers.h"
#include "driver/gpio.h"
#include "esp_log.h"
#include "esp_timer.h"
#include "freertos/FreeRTOS.h" // IWYU pragma: keep
#include "freertos/task.h"
#include <stdio.h>

static const char *TAG = "HELPERS";

// Timing helpers
void delay_ms(uint32_t ms)
{
    vTaskDelay(ms / portTICK_PERIOD_MS);
}

uint32_t get_time_ms(void)
{
    return (uint32_t)(esp_timer_get_time() / 1000);
}

// LED helpers
static bool led_states[GPIO_NUM_MAX] = {false};

void led_init(int gpio_num)
{
    if (gpio_num < 0 || gpio_num >= GPIO_NUM_MAX) {
        ESP_LOGE(TAG, "Invalid GPIO number: %d", gpio_num);
        return;
    }

    gpio_reset_pin(gpio_num);
    gpio_set_direction(gpio_num, GPIO_MODE_OUTPUT);
    led_states[gpio_num] = false;
}

void led_on(int gpio_num)
{
    if (gpio_num < 0 || gpio_num >= GPIO_NUM_MAX) {
        return;
    }
    gpio_set_level(gpio_num, 1);
    led_states[gpio_num] = true;
}

void led_off(int gpio_num)
{
    if (gpio_num < 0 || gpio_num >= GPIO_NUM_MAX) {
        return;
    }
    gpio_set_level(gpio_num, 0);
    led_states[gpio_num] = false;
}

void led_toggle(int gpio_num)
{
    if (gpio_num < 0 || gpio_num >= GPIO_NUM_MAX) {
        return;
    }
    led_states[gpio_num] = !led_states[gpio_num];
    gpio_set_level(gpio_num, led_states[gpio_num] ? 1 : 0);
}

// Debug helpers
void hex_dump(const char *desc, const void *addr, int len)
{
    int i;
    unsigned char buff[17];
    unsigned char *pc = (unsigned char *)addr;

    // Output description if given
    if (desc != NULL) {
        printf("%s:\n", desc);
    }

    // Process every byte in the data
    for (i = 0; i < len; i++) {
        // Multiple of 16 means new line (with line offset)
        if ((i % 16) == 0) {
            // Just don't print ASCII for the zeroth line
            if (i != 0) {
                printf("  %s\n", buff);
            }
            // Output the offset
            printf("  %04x ", i);
        }

        // Now the hex code for the specific character
        printf(" %02x", pc[i]);

        // And store a printable ASCII character for later
        if ((pc[i] < 0x20) || (pc[i] > 0x7e)) {
            buff[i % 16] = '.';
        } else {
            buff[i % 16] = pc[i];
        }
        buff[(i % 16) + 1] = '\0';
    }

    // Pad out last line if not exactly 16 characters
    while ((i % 16) != 0) {
        printf("   ");
        i++;
    }

    // And print the final ASCII bit
    printf("  %s\n", buff);
}

// Math helpers
int map_range(int value, int in_min, int in_max, int out_min, int out_max)
{
    return (value - in_min) * (out_max - out_min) / (in_max - in_min) + out_min;
}

int constrain(int value, int min, int max)
{
    if (value < min) {
        return min;
    } else if (value > max) {
        return max;
    }
    return value;
}

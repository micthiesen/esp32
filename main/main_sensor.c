#include "esp_log.h"
#include "freertos/FreeRTOS.h" // IWYU pragma: keep
#include "freertos/task.h"
#include "helpers.h"

static const char *TAG = "SENSOR_EXAMPLE";

void app_main(void)
{
    ESP_LOGI(TAG, "Sensor reading example starting...");

    while (1) {
        uint32_t timestamp = get_time_ms();
        ESP_LOGI(TAG, "Sensor reading at %u ms", timestamp);
        delay_ms(2000);
    }
}

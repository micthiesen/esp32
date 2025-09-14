#include "esp_log.h"
#include "freertos/FreeRTOS.h" // IWYU pragma: keep
#include "freertos/task.h"
#include "nvs_flash.h"
#include "wifi_helper.h"

static const char *TAG = "WIFI_EXAMPLE";

void app_main(void)
{
    ESP_LOGI(TAG, "WiFi Station Example");

    // Initialize NVS
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ret = nvs_flash_init();
    }
    ESP_ERROR_CHECK(ret);

    // Initialize and connect WiFi using helper
    ret = wifi_helper_connect();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "WiFi connection failed!");
        return;
    }

    // Get and display IP address
    char ip_str[16];
    if (wifi_helper_get_ip_string(ip_str) == ESP_OK) {
        ESP_LOGI(TAG, "WiFi connected! IP: %s", ip_str);
    }

    // Main loop
    while (1) {
        if (wifi_helper_is_connected()) {
            ESP_LOGI(TAG, "WiFi module running... (connected)");
        } else {
            ESP_LOGI(TAG, "WiFi module running... (disconnected)");
        }
        vTaskDelay(5000 / portTICK_PERIOD_MS);
    }
}

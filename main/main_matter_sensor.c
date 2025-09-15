#include "esp_check.h"
#include "esp_err.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h" // IWYU pragma: keep
#include "freertos/task.h"
#include "nvs_flash.h"
#include "wifi_helper.h"
#include <stdbool.h>

static const char *TAG = "MATTER_SENSOR";

// Matter implementation placeholders (will be replaced when ESP-Matter is fully installed)
static esp_err_t matter_sensor_stub_init(void)
{
    ESP_LOGI(TAG, "Matter sensor stub initialized (ESP-Matter SDK not fully available yet)");
    return ESP_OK;
}

static esp_err_t matter_sensor_stub_start(void)
{
    ESP_LOGI(TAG, "Matter sensor stub started (ESP-Matter SDK not fully available yet)");
    return ESP_OK;
}

static esp_err_t matter_sensor_stub_update_temperature(int16_t temperature)
{
    ESP_LOGI(TAG, "Temperature updated: %.2f°C (stub mode)", temperature / 100.0);
    return ESP_OK;
}

static bool matter_sensor_stub_is_commissioned(void)
{
    return false; // Not commissioned in stub mode
}

static void matter_sensor_stub_print_commissioning_info(void)
{
    ESP_LOGI(TAG, "=== Matter Temperature Sensor (Stub Mode) ===");
    ESP_LOGI(TAG, "ESP-Matter SDK not fully installed yet");
    ESP_LOGI(TAG, "This is a simulation showing the project structure");
    ESP_LOGI(TAG, "============================================");
}

// Mock temperature simulation variables
static bool sensor_initialized = false;
static bool sensor_started = false;
static int16_t current_temperature = 2300; // 23.00°C

// Temperature simulation task
static void temperature_simulation_task(void *pvParameters)
{
    ESP_LOGI(TAG, "Temperature simulation task started");

    while (1) {
        if (sensor_started) {
            // Simulate temperature variations (20°C to 26°C)
            static int direction = 1;
            current_temperature += (direction * 10); // ±0.1°C changes

            // Change direction at boundaries
            if (current_temperature >= 2600) { // 26°C
                direction = -1;
            } else if (current_temperature <= 2000) { // 20°C
                direction = 1;
            }

            // Update Matter attribute
            esp_err_t err = matter_sensor_stub_update_temperature(current_temperature);
            if (err == ESP_OK) {
                ESP_LOGI(TAG, "Temperature updated: %.2f°C", current_temperature / 100.0);
            } else {
                ESP_LOGW(TAG, "Failed to update temperature: %s", esp_err_to_name(err));
            }
        }

        vTaskDelay(pdMS_TO_TICKS(5000)); // Update every 5 seconds
    }
}

static esp_err_t matter_sensor_init(void)
{
    if (sensor_initialized) {
        ESP_LOGW(TAG, "Matter sensor already initialized");
        return ESP_OK;
    }

    ESP_LOGI(TAG, "Initializing Matter temperature sensor...");

    // Initialize NVS
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ret = nvs_flash_init();
    }
    ESP_RETURN_ON_ERROR(ret, TAG, "Failed to initialize NVS");

    // Initialize WiFi
    ret = wifi_helper_connect();
    ESP_RETURN_ON_ERROR(ret, TAG, "WiFi connection failed");

    // Get and display IP address
    char ip_str[16];
    if (wifi_helper_get_ip_string(ip_str) == ESP_OK) {
        ESP_LOGI(TAG, "WiFi connected! IP: %s", ip_str);
    }

    // Initialize Matter sensor (stub implementation)
    ret = matter_sensor_stub_init();
    ESP_RETURN_ON_ERROR(ret, TAG, "Failed to initialize Matter sensor");

    sensor_initialized = true;
    ESP_LOGI(TAG, "Matter sensor initialized successfully");

    return ESP_OK;
}

static esp_err_t matter_sensor_start(void)
{
    if (!sensor_initialized) {
        ESP_LOGE(TAG, "Matter sensor not initialized");
        return ESP_ERR_INVALID_STATE;
    }

    if (sensor_started) {
        ESP_LOGW(TAG, "Matter sensor already started");
        return ESP_OK;
    }

    ESP_LOGI(TAG, "Starting Matter temperature sensor...");

    // Start Matter stack
    esp_err_t ret = matter_sensor_stub_start();
    ESP_RETURN_ON_ERROR(ret, TAG, "Failed to start Matter sensor");

    // Create temperature simulation task
    xTaskCreate(temperature_simulation_task, "temp_sim", 4096, NULL, 5, NULL);

    sensor_started = true;
    ESP_LOGI(TAG, "Matter sensor started successfully");

    // Print commissioning information
    matter_sensor_stub_print_commissioning_info();

    return ESP_OK;
}

static bool matter_sensor_is_commissioned(void)
{
    if (!sensor_initialized) {
        return false;
    }

    return matter_sensor_stub_is_commissioned();
}

void app_main(void)
{
    ESP_LOGI(TAG, "Matter WiFi temperature sensor starting...");

    // Initialize Matter sensor
    esp_err_t ret = matter_sensor_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to initialize Matter sensor: %s", esp_err_to_name(ret));
        return;
    }

    // Start Matter sensor
    ret = matter_sensor_start();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to start Matter sensor: %s", esp_err_to_name(ret));
        return;
    }

    ESP_LOGI(TAG, "Matter sensor started successfully!");

    // Main loop - monitor status
    while (1) {
        if (matter_sensor_is_commissioned()) {
            ESP_LOGI(TAG, "Matter device is commissioned and running...");
        } else {
            ESP_LOGI(TAG, "Matter device waiting for commissioning...");
            ESP_LOGI(TAG, "Use Apple Home, Google Home, or other Matter controller to add device");
        }

        vTaskDelay(pdMS_TO_TICKS(30000)); // Status update every 30 seconds
    }
}

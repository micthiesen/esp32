/*
   ESP32 Matter Temperature Sensor Implementation

   This creates a real Matter temperature sensor that can be added to HomeKit
   and other Matter-compatible ecosystems.
*/

#include <app/server/OnboardingCodesUtil.h>
#include <esp_check.h>
#include <esp_err.h>
#include <esp_log.h>
#include <esp_matter.h>
#include <esp_matter_ota.h>
#include <freertos/FreeRTOS.h> // IWYU pragma: keep
#include <freertos/task.h>
#include <nvs_flash.h>
#include <protocols/secure_channel/RendezvousParameters.h>
#include <setup_payload/QRCodeSetupPayloadGenerator.h>
#include <setup_payload/SetupPayload.h>
#include <stdbool.h>
#include <wifi_helper.h>

static const char *TAG = "MATTER_SENSOR";

using namespace esp_matter;
using namespace esp_matter::attribute;
using namespace esp_matter::endpoint;
using namespace chip::app::Clusters;
using namespace chip::DeviceLayer;
using chip::QRCodeBasicSetupPayloadGenerator;

// Temperature simulation variables
static bool sensor_initialized = false;
static bool sensor_started = false;
static int16_t current_temperature = 2300; // 23.00°C
static uint16_t temperature_endpoint_id = 0;

// Matter attribute update callback for temperature
static void update_temperature_attribute(int16_t temperature_celsius_x100)
{
    if (temperature_endpoint_id == 0) {
        ESP_LOGW(TAG, "Temperature endpoint not initialized yet");
        return;
    }

    // Schedule the attribute update on the Matter thread
    SystemLayer().ScheduleLambda([temperature_celsius_x100]() {
        attribute_t *attribute =
            attribute::get(temperature_endpoint_id, TemperatureMeasurement::Id,
                           TemperatureMeasurement::Attributes::MeasuredValue::Id);

        if (attribute == nullptr) {
            ESP_LOGE(TAG, "Failed to get temperature attribute");
            return;
        }

        esp_matter_attr_val_t val = esp_matter_invalid(NULL);
        attribute::get_val(attribute, &val);
        val.val.i16 = temperature_celsius_x100;

        esp_err_t err =
            attribute::update(temperature_endpoint_id, TemperatureMeasurement::Id,
                              TemperatureMeasurement::Attributes::MeasuredValue::Id, &val);

        if (err != ESP_OK) {
            ESP_LOGE(TAG, "Failed to update temperature attribute: %s", esp_err_to_name(err));
        } else {
            ESP_LOGI(TAG, "Temperature updated: %.2f°C", temperature_celsius_x100 / 100.0);
        }
    });
}

// Temperature simulation task
static void temperature_simulation_task(void *pvParameters)
{
    ESP_LOGI(TAG, "Temperature simulation task started");

    while (1) {
        if (sensor_started && temperature_endpoint_id != 0) {
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
            update_temperature_attribute(current_temperature);
        }

        vTaskDelay(pdMS_TO_TICKS(5000)); // Update every 5 seconds
    }
}

// Matter event callback
static void app_event_cb(const ChipDeviceEvent *event, intptr_t arg)
{
    switch (event->Type) {
    case DeviceEventType::kCommissioningComplete:
        ESP_LOGI(TAG, "Commissioning complete - device paired successfully!");
        break;

    case DeviceEventType::kFailSafeTimerExpired:
        ESP_LOGI(TAG, "Commissioning failed, fail safe timer expired");
        break;

    case DeviceEventType::kFabricRemoved:
        ESP_LOGI(TAG, "Fabric removed successfully");
        break;

    case DeviceEventType::kBLEDeinitialized:
        ESP_LOGI(TAG, "BLE deinitialized and memory reclaimed");
        break;

    default:
        break;
    }
}

// Matter identification callback
static esp_err_t app_identification_cb(identification::callback_type_t type, uint16_t endpoint_id,
                                       uint8_t effect_id, uint8_t effect_variant, void *priv_data)
{
    ESP_LOGI(TAG, "Identification callback: type: %u, effect: %u, variant: %u", type, effect_id,
             effect_variant);
    return ESP_OK;
}

// Matter attribute update callback
static esp_err_t app_attribute_update_cb(attribute::callback_type_t type, uint16_t endpoint_id,
                                         uint32_t cluster_id, uint32_t attribute_id,
                                         esp_matter_attr_val_t *val, void *priv_data)
{
    // Temperature sensor is read-only, so no writes expected
    return ESP_OK;
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

    // Create a Matter node
    node::config_t node_config;
    node_t *node = node::create(&node_config, app_attribute_update_cb, app_identification_cb);
    if (node == nullptr) {
        ESP_LOGE(TAG, "Failed to create Matter node");
        return ESP_FAIL;
    }

    // Create temperature sensor endpoint
    temperature_sensor::config_t temp_sensor_config;
    endpoint_t *temp_sensor_ep =
        temperature_sensor::create(node, &temp_sensor_config, ENDPOINT_FLAG_NONE, NULL);
    if (temp_sensor_ep == nullptr) {
        ESP_LOGE(TAG, "Failed to create temperature sensor endpoint");
        return ESP_FAIL;
    }

    // Store endpoint ID for attribute updates
    temperature_endpoint_id = endpoint::get_id(temp_sensor_ep);
    ESP_LOGI(TAG, "Temperature sensor endpoint created with ID: %u", temperature_endpoint_id);

    // Start Matter stack
    esp_err_t err = esp_matter::start(app_event_cb);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to start Matter stack: %s", esp_err_to_name(err));
        return err;
    }

    // Create temperature simulation task
    xTaskCreate(temperature_simulation_task, "temp_sim", 4096, NULL, 5, NULL);

    sensor_started = true;
    ESP_LOGI(TAG, "Matter sensor started successfully");

    // ESP-Matter will automatically handle commissioning window

    return ESP_OK;
}

static void print_commissioning_info(void)
{
    ESP_LOGI(TAG, "=== Matter Temperature Sensor ===");
    ESP_LOGI(TAG, "Device Type: Temperature Sensor");
    ESP_LOGI(TAG, "Vendor ID: Test Vendor (0xFFF1)");
    ESP_LOGI(TAG, "Product ID: Test Product (0x8000)");

    ESP_LOGI(TAG, "Status: Waiting for commissioning");
    ESP_LOGI(TAG, "");
    ESP_LOGI(TAG, "=== COMMISSIONING CODES ===");

    // Generate QR code and manual pairing code for WiFi (OnNetwork)
    char qrCodeBuffer[QRCodeBasicSetupPayloadGenerator::kMaxQRCodeBase38RepresentationLength + 1];
    char manualCodeBuffer[QRCodeBasicSetupPayloadGenerator::kMaxQRCodeBase38RepresentationLength +
                          1];

    chip::MutableCharSpan qrCode(qrCodeBuffer, sizeof(qrCodeBuffer));
    chip::MutableCharSpan manualCode(manualCodeBuffer, sizeof(manualCodeBuffer));

    // Use OnNetwork rendezvous for WiFi devices
    chip::RendezvousInformationFlags rendezvousFlags(chip::RendezvousInformationFlag::kOnNetwork);

    // Get QR code
    if (GetQRCode(qrCode, rendezvousFlags) == CHIP_NO_ERROR) {
        ESP_LOGI(TAG, "");
        ESP_LOGI(TAG, "QR Code: %s", qrCode.data());
        ESP_LOGI(TAG, "");
        ESP_LOGI(TAG, "Copy/paste this URL to see QR code in browser:");

        // Generate QR code URL
        char qrCodeUrl[512];
        if (GetQRCodeUrl(qrCodeUrl, sizeof(qrCodeUrl), qrCode) == CHIP_NO_ERROR) {
            ESP_LOGI(TAG, "%s", qrCodeUrl);
        }
    } else {
        ESP_LOGI(TAG, "Failed to generate QR code");
    }

    // Get manual pairing code
    if (GetManualPairingCode(manualCode, rendezvousFlags) == CHIP_NO_ERROR) {
        ESP_LOGI(TAG, "");
        ESP_LOGI(TAG, "Manual setup code: %s", manualCode.data());
    } else {
        ESP_LOGI(TAG, "Failed to generate manual pairing code");
    }

    ESP_LOGI(TAG, "");
    ESP_LOGI(TAG, "Add to HomeKit: Scan QR code or enter manual setup code");
    ESP_LOGI(TAG, "===========================");
    ESP_LOGI(TAG, "==================================");
}

extern "C" void app_main(void)
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

    // Print initial commissioning info
    vTaskDelay(pdMS_TO_TICKS(2000)); // Wait for initialization to complete
    print_commissioning_info();

    // Main loop - monitor status
    while (1) {
        ESP_LOGI(TAG, "Current temperature: %.2f°C", current_temperature / 100.0);
        vTaskDelay(pdMS_TO_TICKS(30000)); // Status update every 30 seconds
    }
}

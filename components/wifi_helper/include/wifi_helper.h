#pragma once

#include "esp_event.h" // IWYU pragma: keep
#include "esp_wifi.h"  // IWYU pragma: keep

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief WiFi Helper Component
 *
 * Provides simplified WiFi configuration and connection management.
 * Requires wifi_config.h to be created from wifi_config.h.template
 */

/**
 * @brief Initialize and connect to WiFi station
 *
 * This function will:
 * 1. Validate WiFi configuration exists
 * 2. Initialize WiFi subsystem
 * 3. Connect to configured access point
 * 4. Wait for connection or timeout
 *
 * @return ESP_OK on success, error code on failure
 */
esp_err_t wifi_helper_connect(void);

/**
 * @brief Check if WiFi is connected
 *
 * @return true if connected, false otherwise
 */
bool wifi_helper_is_connected(void);

/**
 * @brief Get current IP address as string
 *
 * @param ip_str Buffer to store IP string (minimum 16 chars)
 * @return ESP_OK on success, ESP_FAIL if not connected
 */
esp_err_t wifi_helper_get_ip_string(char *ip_str);

#ifdef __cplusplus
}
#endif

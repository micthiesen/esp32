idf_component_register(
    SRCS "src/wifi_helper.c"
    INCLUDE_DIRS "include" "${CMAKE_SOURCE_DIR}/main"
    REQUIRES esp_wifi esp_event nvs_flash esp_netif
    PRIV_REQUIRES lwip
)
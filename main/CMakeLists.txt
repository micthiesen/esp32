# Main component CMakeLists.txt

# Define the main source file (set by root CMakeLists.txt)
idf_component_register(
    SRCS "${MAIN_SOURCE}"
    INCLUDE_DIRS "."
    REQUIRES helpers wifi_helper esp_wifi esp_event nvs_flash
    PRIV_REQUIRES esp_netif lwip
)
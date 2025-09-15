# Main component CMakeLists.txt

# Define the main source file (set by root CMakeLists.txt)
# Set component requirements based on selected module
if(MAIN_MODULE STREQUAL "matter")
    set(COMPONENT_REQUIRES helpers wifi_helper esp_wifi esp_event nvs_flash)
    set(COMPONENT_PRIV_REQUIRES esp_netif lwip)
else()
    set(COMPONENT_REQUIRES helpers wifi_helper esp_wifi esp_event nvs_flash)
    set(COMPONENT_PRIV_REQUIRES esp_netif lwip)
endif()

idf_component_register(
    SRCS "${MAIN_SOURCE}"
    INCLUDE_DIRS "."
    REQUIRES ${COMPONENT_REQUIRES}
    PRIV_REQUIRES ${COMPONENT_PRIV_REQUIRES}
)
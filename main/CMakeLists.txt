# main/CMakeLists.txt

# Define the main source file (set by root CMakeLists.txt)
# Set component requirements based on selected module

# Base deps
set(COMPONENT_REQUIRES helpers wifi_helper esp_wifi esp_event nvs_flash)
set(COMPONENT_PRIV_REQUIRES esp_netif lwip)

# Contains "matter" (literal substring)
string(FIND "${MAIN_MODULE}" "matter" _matter_pos)
if(NOT _matter_pos EQUAL -1)
    # append any Matter-specific deps here if needed, e.g. esp_matter
    list(APPEND COMPONENT_REQUIRES esp_matter)
endif()

idf_component_register(
    SRCS "${MAIN_SOURCE}"
    INCLUDE_DIRS "."
    REQUIRES ${COMPONENT_REQUIRES}
    PRIV_REQUIRES ${COMPONENT_PRIV_REQUIRES}
)

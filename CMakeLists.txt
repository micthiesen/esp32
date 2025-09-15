cmake_minimum_required(VERSION 3.16)

# --- Configurable project name
set(PROJECT_NAME "esp32_multi" CACHE STRING "Project name")

# --- Reconfigure when .main_module changes (explicit + portable)
# Avoid relying on configure_file for this; use CMAKE_CONFIGURE_DEPENDS.
set(_MAIN_MODULE_FILE "${CMAKE_CURRENT_SOURCE_DIR}/.main_module")
set_property(DIRECTORY APPEND PROPERTY CMAKE_CONFIGURE_DEPENDS "${_MAIN_MODULE_FILE}")

if(EXISTS "${_MAIN_MODULE_FILE}")
    file(READ "${_MAIN_MODULE_FILE}" MAIN_MODULE)
    string(STRIP "${MAIN_MODULE}" MAIN_MODULE)

    # Strictly validate to avoid weird filenames / injection
    if(NOT MAIN_MODULE MATCHES "^[A-Za-z0-9_]+$")
        message(FATAL_ERROR ".main_module contains invalid chars: '${MAIN_MODULE}'")
    endif()

    set(MAIN_SOURCE "main_${MAIN_MODULE}.c")
    message(STATUS "Using main module: ${MAIN_SOURCE}")
else()
    message(FATAL_ERROR "No .main_module file found at ${_MAIN_MODULE_FILE}")
endif()

# Verify the chosen main file exists in the 'main' component dir
if(NOT EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/main/${MAIN_SOURCE}")
    message(FATAL_ERROR "Selected source not found: main/${MAIN_SOURCE}")
endif()

# --- Optional: enable ESP-Matter only when module name contains 'matter'
# Prefer literal substring search over regex if you mean “contains”.
string(FIND "${MAIN_MODULE}" "matter" _matter_pos)
if(NOT _matter_pos EQUAL -1)
    set(ESP_MATTER_PATH "${CMAKE_CURRENT_SOURCE_DIR}/esp-matter" CACHE PATH "Path to ESP-Matter")
    set(ENV{ESP_MATTER_PATH} "${ESP_MATTER_PATH}")  # if esp-matter expects ENV
    # EXTRA_COMPONENT_DIRS must be set BEFORE including project.cmake
    set(EXTRA_COMPONENT_DIRS "${ESP_MATTER_PATH}/components")
    message(STATUS "Matter module detected; adding ESP-Matter components from: ${EXTRA_COMPONENT_DIRS}")
endif()

# --- Pass the selected file to the 'main' component via cache var
# The main/CMakeLists.txt should use: idf_component_register(SRCS "${MAIN_SOURCE}" ...)
set(MAIN_SOURCE "${MAIN_SOURCE}" CACHE STRING "Selected main source for 'main' component" FORCE)

# --- ESP-IDF glue (order matters: set vars -> include project.cmake -> project())
include($ENV{IDF_PATH}/tools/cmake/project.cmake)

# Limit to specific components if desired (optional)
set(COMPONENTS main)

project(${PROJECT_NAME})

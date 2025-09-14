cmake_minimum_required(VERSION 3.16)

# Allow customization of the project
set(PROJECT_NAME "esp32_multi" CACHE STRING "Project name")

# Set the main source file based on .main_module file
# Configure the .main_module file to trigger reconfiguration when it changes
configure_file("${CMAKE_CURRENT_SOURCE_DIR}/.main_module"
               "${CMAKE_CURRENT_BINARY_DIR}/.main_module_configured"
               COPYONLY)

if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/.main_module")
    file(READ "${CMAKE_CURRENT_SOURCE_DIR}/.main_module" MAIN_MODULE)
    string(STRIP "${MAIN_MODULE}" MAIN_MODULE)
    set(MAIN_SOURCE "main_${MAIN_MODULE}.c")
    message(STATUS "Using main module from file: ${MAIN_SOURCE}")
else()
    set(MAIN_SOURCE "main_blink.c")
    set(MAIN_MODULE "blink")
    message(STATUS "No .main_module file found, using default: ${MAIN_SOURCE}")
    file(WRITE "${CMAKE_CURRENT_SOURCE_DIR}/.main_module" "blink")
endif()

# Include ESP-IDF
include($ENV{IDF_PATH}/tools/cmake/project.cmake)

# Set the components to include
set(COMPONENTS main)

project(${PROJECT_NAME})
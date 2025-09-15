cmake_minimum_required(VERSION 3.16)

# Resolve IDF_PATH to submodule if not exported by user shell
if(NOT DEFINED ENV{IDF_PATH})
  set(ENV{IDF_PATH} "${CMAKE_SOURCE_DIR}/esp-idf")
endif()

# --- Discover available mains
file(GLOB _MAIN_C_SOURCES   CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/main/main_*.c")
file(GLOB _MAIN_CPP_SOURCES CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/main/main_*.cpp")

set(_MODULE_NAMES "")
foreach(_f IN LISTS _MAIN_C_SOURCES _MAIN_CPP_SOURCES)
  string(REGEX REPLACE ".*/main_([^/]+)\\.(c|cpp)$" "\\1" _mod "${_f}")
  list(FIND _MODULE_NAMES "${_mod}" _idx)
  if(_idx EQUAL -1)
    list(APPEND _MODULE_NAMES "${_mod}")
  endif()
endforeach()

if(NOT _MODULE_NAMES)
  message(FATAL_ERROR "No main/main_*.c(pp) found")
endif()

# User-selectable module: set value and offer choices in cache
set(MAIN_MODULE "${_MODULE_NAMES}" CACHE STRING "Select main_ module (basename without extension)")
set_property(CACHE MAIN_MODULE PROPERTY STRINGS ${_MODULE_NAMES})

# Resolve selected source (prefer .cpp over .c if both exist)
set(_cand_cpp "${CMAKE_CURRENT_SOURCE_DIR}/main/main_${MAIN_MODULE}.cpp")
set(_cand_c   "${CMAKE_CURRENT_SOURCE_DIR}/main/main_${MAIN_MODULE}.c")
if(EXISTS "${_cand_cpp}")
  set(MAIN_SOURCE "main_${MAIN_MODULE}.cpp")
elseif(EXISTS "${_cand_c}")
  set(MAIN_SOURCE "main_${MAIN_MODULE}.c")
else()
  message(FATAL_ERROR "MAIN_MODULE='${MAIN_MODULE}' not found as .c or .cpp under main/")
endif()
set(MAIN_SOURCE "${MAIN_SOURCE}" CACHE STRING "Resolved source for selected module" FORCE)

# If module name contains "matter", enable esp-matter components
string(FIND "${MAIN_MODULE}" "matter" _mm_pos)
if(NOT _mm_pos EQUAL -1)
  set(ESP_MATTER_PATH "${CMAKE_CURRENT_SOURCE_DIR}/esp-matter" CACHE PATH "Path to ESP-Matter")
  list(APPEND EXTRA_COMPONENT_DIRS "${ESP_MATTER_PATH}/components")
endif()

# ESP-IDF integration
include($ENV{IDF_PATH}/tools/cmake/project.cmake)

project(esp32_multi)

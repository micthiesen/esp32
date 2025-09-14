idf_component_register(
    SRCS "src/helpers.c"
    INCLUDE_DIRS "include"
    REQUIRES log driver esp_timer
)
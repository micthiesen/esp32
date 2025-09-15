# ESP32 Development Guidelines for Claude

This document contains important instructions and guidelines for Claude when working on this ESP32 project.

## Project Structure and Conventions

### Dynamic Module Selection System
- Main modules are automatically discovered from `main/main_*.c` files
- Use `./select_main.sh` to see and select available modules
- The script extracts descriptions from ESP_LOGI statements in the code
- No hardcoded module lists to maintain - fully dynamic

### Component Architecture
- **helpers/** - Utility functions (timing, LED control, math, debug)
- **wifi_helper/** - WiFi connection management with secure credential system
- Each component has proper CMakeLists.txt and clear API
- Components can include external directories (e.g., main/ for wifi_config.h)

### Code Style Requirements

#### Header Ordering
- Always include `freertos/FreeRTOS.h` BEFORE `freertos/task.h`
- Add `// IWYU pragma: keep` for headers flagged as unused but required
- Follow ESP-IDF include conventions

#### WiFi Configuration
- WiFi credentials go in `main/wifi_config.h` (NOT version controlled)
- Use `main/wifi_config.h.template` as the template
- Components needing wifi_config.h must include main/ in INCLUDE_DIRS:
  ```cmake
  INCLUDE_DIRS "include" "${CMAKE_SOURCE_DIR}/main"
  ```
- Include compile-time validation in WiFi components

#### Formatting
- Use clang-format for all C/C++ code
- Configuration is in `.clang-format`
- Zed automatically formats on save

## Build and Development Workflow

### Standard Development Commands
```bash
# Select module (automatically discovers available modules)
./select_main.sh

# Build current module (debug)
source ./esp-idf/export.sh && idf.py build

# Flash to device
source ./esp-idf/export.sh && idf.py flash

# Monitor output
source ./esp-idf/export.sh && idf.py monitor

## Production Builds
- Use "Build & Flash (Production)" Zed task for optimized production deployment
- Each task automatically uses its appropriate configuration (debug tasks use debug config, production task uses production config)
- Production config: size optimization, error-only logging, assertions disabled
- No manual config management needed - tasks handle everything

### Zed Editor Integration
- Tasks are configured in `.zed/tasks.json`
- Use Cmd+Shift+P → Tasks to access ESP32 operations
- Tasks auto-close on success, remain open on failure
- clangd LSP is configured with ESP-IDF paths and custom rule exclusions

### Adding New Main Modules
1. Create `main/main_yourmodule.c`
2. Include app_main() function
3. Add descriptive ESP_LOGI line for better auto-generated descriptions:
   ```c
   ESP_LOGI(TAG, "Your module description starting...");
   ```
4. Run `./select_main.sh` - your module will be auto-discovered
5. Build and test

## Troubleshooting

### Common Issues
1. **Module switching**: If builds seem stale, check `.main_module` file
2. **WiFi compilation errors**: Ensure `main/wifi_config.h` exists and component includes main/
3. **clangd warnings**: Most pedantic warnings are filtered out in `.clangd` config
4. **Build failures**: Clean with `rm -rf build` or use `idf.py fullclean`
5. **Linting errors**: The script handles GCC/clang compatibility automatically

### ESP-IDF Environment
- ESP-IDF is included as a git submodule at `./esp-idf/`
- Always source `./esp-idf/export.sh` before running idf.py commands
- Use `git submodule update --recursive` if ESP-IDF seems outdated

## Security Guidelines

### WiFi Credentials
- **NEVER** commit `main/wifi_config.h` to version control
- Always use the template system for configuration
- Include compile-time validation in WiFi components
- Components accessing wifi_config.h must add main/ to include paths

### Code Review Checklist
- [ ] No credentials or secrets in code
- [ ] Proper FreeRTOS header ordering
- [ ] Component CMakeLists.txt updated if needed
- [ ] WiFi configuration properly templated if applicable
- [ ] New modules have descriptive ESP_LOGI statements

## Development Best Practices

1. **Always test with real hardware** - Don't assume code works without flashing
2. **Use component architecture** - Extract reusable functionality
3. **Follow ESP-IDF patterns** - Use established ESP-IDF conventions
4. **Monitor memory usage** - ESP32 has limited RAM
5. **Handle errors properly** - Use ESP_ERROR_CHECK and proper error handling
6. **Log appropriately** - Use ESP_LOG* macros with appropriate levels
7. **Document modules** - Include descriptive ESP_LOGI statements for auto-discovery

## Component Development

### Creating New Components
1. Create directory in `components/yourcomponent/`
2. Add `CMakeLists.txt` with proper dependencies
3. Include external directories if needed:
   ```cmake
   INCLUDE_DIRS "include" "${CMAKE_SOURCE_DIR}/main"
   ```
4. Provide clear API in include files
5. Test integration with existing modules

### WiFi Components
- Must include main/ directory for wifi_config.h access
- Include compile-time validation
- Handle connection failures gracefully
- Provide status checking functions

## Notes for Claude
- The linting script automatically filters incompatible compiler flags
- Custom clang-tidy rules balance code quality with ESP32 practicality
- Module selection is fully dynamic - no maintenance needed when adding modules
- All WiFi functionality uses the secure credential system
- Components can access main/ directory for configuration files

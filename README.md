# ESP32 Multi-Module Project

A modular ESP32 development environment with support for multiple main modules, configured for the Zed editor.

## Included Components

- **ESP-IDF v5.4** (as git submodule - ensures version consistency)
- **Build system** - CMake with multi-module support
- **Code quality** - clang-format, clangd configuration
- **Zed integration** - Tasks, settings, and LSP setup
- **Self-contained** - No global ESP-IDF installation required

## Project Structure

```
esp32/
├── .zed/                 # Zed editor configuration
│   ├── settings.json     # LSP and formatting settings
│   └── tasks.json        # Build/flash/monitor tasks
├── main/                 # Main modules directory
│   ├── main_blink.c      # LED blink example
│   ├── main_wifi.c       # WiFi connection example
│   └── main_sensor.c     # Sensor reading example
├── components/           # Shared components
│   └── helpers/          # Helper functions library
├── CMakeLists.txt        # Root CMake configuration
├── .clang-format         # C/C++ formatting rules
├── .clangd               # LSP configuration
└── select_main.sh        # Module selection script
```

## Setup Instructions

### 1. Clone with Submodules

When cloning this repository, use `--recursive` to get the ESP-IDF submodule:

```bash
git clone --recursive <your-repo-url>
cd esp32
```

Or if you already cloned without `--recursive`:

```bash
git submodule update --init --recursive
```

### 2. Install ESP-IDF Tools

```bash
cd esp-idf
./install.sh esp32,esp32s3,esp32c3
cd ..
```

### 3. Set up Environment Alias

Add this to your shell profile (~/.zshrc or ~/.bashrc):

```bash
alias get_idf='. ./esp-idf/export.sh'
```

### 4. Initialize ESP-IDF for Each Session

Before working with ESP32, run (from project root):
```bash
get_idf
```

## Building Projects

### Method 1: Using Selection Script (Recommended)

```bash
./select_main.sh
# Follow the prompts to select a module
# The selection is saved and persists across sessions
```

### Method 2: Manual Module Selection

```bash
# Write the module name directly to the file
echo "wifi" > .main_module

# Then build normally
get_idf
idf.py build
```

### Method 3: Direct Build

The system automatically uses the currently selected module:

```bash
get_idf
idf.py build  # Uses whatever is in .main_module
```

## Zed Editor Tasks

Open the Command Palette (Cmd+Shift+P) and search for "task" to see available tasks:

### Build & Flash Tasks
- **Build** - Compile the current module
- **Flash** - Upload to ESP32
- **Monitor** - View serial output
- **Build & Flash** - Compile and upload
- **Build, Flash & Monitor** - All in one
- **Clean** - Remove build artifacts

### Module Management
- **Select Main Module** - Interactive module selection
- **Show Current Module** - Display currently selected module

### Configuration
- **Menuconfig** - Configure ESP32 settings
- **Set Target** - Switch between ESP32/ESP32-S3/ESP32-C3

### Analysis & Debug
- **Size Analysis** - Analyze binary size
- **Component Size Analysis** - Detailed size breakdown
- **Open Serial Monitor** - Monitor serial output
- **Erase Flash** - Completely erase flash memory

## Common Commands

```bash
# Set target chip
idf.py set-target esp32    # or esp32s3, esp32c3

# Configure project
idf.py menuconfig

# Build project
idf.py build

# Flash to device
idf.py flash

# Monitor serial output
idf.py monitor

# Combined commands
idf.py build flash monitor

# Clean build
idf.py fullclean

# Size analysis
idf.py size
idf.py size-components
```

## Module Selection System

The project uses a **file-based module selection** system:

- Current module is stored in `.main_module` file
- This file is user-specific (not committed to git)
- All build commands automatically use the selected module
- Module selection persists across terminal sessions and IDE restarts

### Current Module Status

```bash
# Check current module
cat .main_module

# Or use the task: "Show Current Module" in Zed
```

## Adding New Main Modules

1. Create a new file `main/main_yourmodule.c`
2. Include the app_main() function
3. Select your module: `./select_main.sh` or `echo "yourmodule" > .main_module`
4. Build: `idf.py build`

## WiFi Module Configuration

The WiFi module uses a secure configuration system to keep credentials out of version control:

1. **Copy the template file:**
   ```bash
   cp main/wifi_config.h.template main/wifi_config.h
   ```

2. **Edit your WiFi credentials:**
   ```bash
   # Edit with your preferred editor
   nano main/wifi_config.h
   # or
   code main/wifi_config.h
   ```

3. **Update the configuration:**
   ```c
   #define WIFI_SSID "Your_Actual_WiFi_Name"
   #define WIFI_PASS "Your_Actual_Password"
   ```

4. **Build and flash:**
   ```bash
   idf.py build flash monitor
   ```

**Note:** The `wifi_config.h` file is git-ignored to keep your credentials secure. Each developer needs to create their own configuration file.

### Advanced WiFi Settings

You can customize additional WiFi parameters in `wifi_config.h`:

```c
// Connection settings
#define WIFI_MAX_RETRY 10                    // Retry attempts
#define WIFI_CONNECT_TIMEOUT_MS 15000        // Timeout in milliseconds

// Security settings
#define WIFI_AUTH_MODE WIFI_AUTH_WPA2_PSK    // Authentication mode
```

Available authentication modes:
- `WIFI_AUTH_OPEN` - No security (not recommended)
- `WIFI_AUTH_WEP` - WEP (legacy, not recommended)
- `WIFI_AUTH_WPA_PSK` - WPA-PSK
- `WIFI_AUTH_WPA2_PSK` - WPA2-PSK (recommended)
- `WIFI_AUTH_WPA2_WPA3_PSK` - WPA2/WPA3 mixed mode

## Troubleshooting

### Port Access Issues
If you get permission errors accessing the serial port:
```bash
sudo usermod -a -G dialout $USER  # Linux
# On macOS, the user should already have access
```

### Build Errors
1. Ensure ESP-IDF is sourced: `get_idf`
2. Clean and rebuild: `idf.py fullclean && idf.py build`
3. Check target matches your board: `idf.py set-target esp32`

### Zed LSP Issues
1. Ensure you've run the initial setup: `get_idf` from the project root
2. Restart Zed from the project directory: `zed .`
3. The project now uses a local ESP-IDF submodule for consistency

### Module Selection Issues
- If build fails with "No such file", check current module: `cat .main_module`
- Run `./select_main.sh` to select a valid module
- Verify the main file exists: `ls main/main_$(cat .main_module).c`

## Helper Functions

The `helpers` component provides utility functions:

- **Timing**: `delay_ms()`, `get_time_ms()`
- **LED Control**: `led_init()`, `led_on()`, `led_off()`, `led_toggle()`
- **Debug**: `hex_dump()`
- **Math**: `map_range()`, `constrain()`

Include in your module:
```c
#include "helpers.h"
```
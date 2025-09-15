#!/bin/bash

# Script to select which main module to build
# Usage: ./select_main.sh [production]

# Check if production mode requested
PRODUCTION_MODE=""
if [ "$1" = "production" ]; then
    PRODUCTION_MODE="production"
    echo "ESP32 Main Module Selector (Production Mode)"
    echo "============================================="
else
    echo "ESP32 Main Module Selector"
    echo "=========================="
fi
echo ""

# Show current selection if it exists
PREVIOUS_MODULE=""
if [ -f ".main_module" ]; then
    PREVIOUS_MODULE=$(cat .main_module)
    echo "Current module: $PREVIOUS_MODULE (main_${PREVIOUS_MODULE}.c)"
    echo ""
fi

# Dynamically discover available modules
echo "Discovering available modules..."
MODULES=()
MODULE_FILES=()

# Find all main_*.c files and extract module names
for file in main/main_*.c; do
    if [ -f "$file" ]; then
        # Extract module name: main/main_blink.c -> blink
        module_name=$(basename "$file" .c | sed 's/^main_//')
        MODULES+=("$module_name")
        MODULE_FILES+=("$file")
    fi
done

# Check if any modules were found
if [ ${#MODULES[@]} -eq 0 ]; then
    echo "Error: No main_*.c files found in main/ directory!"
    exit 1
fi

echo ""
echo "Available modules:"
for i in "${!MODULES[@]}"; do
    num=$((i + 1))
    module="${MODULES[$i]}"
    file="${MODULE_FILES[$i]}"

    # Try to extract description from the file
    description=""
    if [ -f "$file" ]; then
        # Look for ESP_LOGI lines that might describe what the module does
        log_line=$(grep "ESP_LOGI.*starting\|ESP_LOGI.*example" "$file" | head -1 | sed 's/.*ESP_LOGI.*"\([^"]*\)".*/\1/' || echo "")
        if [ -n "$log_line" ] && [ "$log_line" != "$(grep "ESP_LOGI.*starting\|ESP_LOGI.*example" "$file" | head -1)" ]; then
            description="$log_line"
        else
            # Fallback: try to infer from filename
            case "$module" in
                blink) description="LED blinking example" ;;
                wifi) description="WiFi connection example" ;;
                sensor) description="Sensor reading example" ;;
                matter) description="Matter WiFi temperature sensor" ;;
                *) description="ESP32 application" ;;
            esac
        fi
    fi

    echo "  $num) $module - $description"
done
echo ""

read -p "Select module (1-${#MODULES[@]}) or enter name directly: " choice

# Handle the choice
MODULE=""
if [[ "$choice" =~ ^[0-9]+$ ]] && [ "$choice" -ge 1 ] && [ "$choice" -le ${#MODULES[@]} ]; then
    # Numeric choice
    index=$((choice - 1))
    MODULE="${MODULES[$index]}"
elif [ -n "$choice" ]; then
    # Direct name entry
    MODULE="$choice"
else
    echo "No selection made. Exiting."
    exit 0
fi

# Check if the module file exists
if [ ! -f "main/main_${MODULE}.c" ]; then
    echo "Error: main/main_${MODULE}.c not found!"
    echo "Available files:"
    ls main/main_*.c 2>/dev/null | sed 's|main/main_||' | sed 's|\.c||' | sed 's/^/  - /'
    exit 1
fi

# Write the selection to the .main_module file
echo "$MODULE" > .main_module

echo ""
echo "✓ Selected module: $MODULE (main_${MODULE}.c)"
echo "✓ Module selection saved to .main_module"

# Handle configuration selection based on module and mode
echo ""
if [ "$PRODUCTION_MODE" = "production" ]; then
    echo "🔧 Configuring production build for $MODULE module..."
    # Production mode - use production config
    if [ -f "configs/sdkconfig.production" ]; then
        cp configs/sdkconfig.production sdkconfig
        echo "✓ Applied production configuration (sdkconfig)"
    else
        echo "⚠ Warning: configs/sdkconfig.production not found, using debug config"
        exit 1;
    fi
else
    echo "🔧 Configuring build for $MODULE module..."

    if [ "$MODULE" = "matter" ]; then
        # Matter module needs special configuration
        if [ -f "configs/sdkconfig.matter" ]; then
            cp configs/sdkconfig.matter sdkconfig
            echo "✓ Applied Matter-specific configuration (sdkconfig)"
        else
            echo "⚠ Warning: configs/sdkconfig.matter not found, using default config"
            exit 1;
        fi

        echo "ℹ Matter module requires:"
        echo "  - WiFi credentials in main/wifi_config.h"
        echo "  - ESP-Matter SDK (if not installed, run: cd esp-matter && ./install.sh)"
        echo "  - 4MB flash size minimum"
    else
        # For non-Matter modules, use debug configuration
        if [ -f "configs/sdkconfig.debug" ]; then
            cp configs/sdkconfig.debug sdkconfig
            echo "✓ Applied debug configuration (sdkconfig)"
        else
            echo "⚠ Warning: configs/sdkconfig.debug not found"
            exit 1;
        fi
    fi
fi

  echo ""
  echo "🧹 Module changed from '$PREVIOUS_MODULE' to '$MODULE'"
  echo "🧹 Cleaning build directory to ensure fresh build..."

  if [ -d "build" ]; then
      rm -rf build
      echo "✓ Build directory cleaned"
  else
      echo "ℹ Build directory was already clean"
  fi

echo ""
echo "You can now build with:"
echo "  source ./esp-idf/export.sh"
echo "  idf.py build"
echo ""
echo "Or use Zed tasks: Cmd+Shift+P → 'task' → 'Build'"
echo ""
echo "ℹ For production builds, use: './select_main.sh production' or the 'Build & Flash (Production)' task"

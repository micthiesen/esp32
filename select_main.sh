#!/bin/bash

# Script to select which main module to build

echo "ESP32 Main Module Selector"
echo "=========================="
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

# Check if we're switching to a different module
MODULE_CHANGED=false
if [ "$PREVIOUS_MODULE" != "" ] && [ "$PREVIOUS_MODULE" != "$MODULE" ]; then
    MODULE_CHANGED=true
fi

# Write the selection to the .main_module file
echo "$MODULE" > .main_module

echo ""
echo "✓ Selected module: $MODULE (main_${MODULE}.c)"
echo "✓ Module selection saved to .main_module"

# Auto-clean if module changed
if [ "$MODULE_CHANGED" = true ]; then
    echo ""
    echo "🧹 Module changed from '$PREVIOUS_MODULE' to '$MODULE'"
    echo "🧹 Cleaning build directory to ensure fresh build..."

    if [ -d "build" ]; then
        rm -rf build
        echo "✓ Build directory cleaned"
    else
        echo "ℹ Build directory was already clean"
    fi
fi

echo ""
echo "You can now build with:"
echo "  source ./esp-idf/export.sh"
echo "  idf.py build"
echo ""
echo "Or use Zed tasks: Cmd+Shift+P → 'task' → 'Build'"

#!/usr/bin/env bash
set -euo pipefail

# Repo root
repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$repo_root"

# Locate esp-idf (env first, then common submodule paths)
IDF_PATH="${HOME}/Code/esp-idf"
if [[ -z "$IDF_PATH" || ! -f "$IDF_PATH/tools/idf.py" ]]; then
  echo "Could not locate esp-idf (set IDF_PATH or install at ~/Code/esp-idf/)" >&2
  exit 1
fi
export IDF_PATH

# Optional: set up toolchain/venv in PATH
if [[ -f "$IDF_PATH/export.sh" ]]; then
  # shellcheck disable=SC1090
  . "$IDF_PATH/export.sh" >/dev/null
fi

# Show current configuration
echo "=== Current Configuration ==="
current_target="Unknown"
if [[ -f "sdkconfig" ]]; then
  current_target=$(grep 'CONFIG_IDF_TARGET=' sdkconfig | cut -d'=' -f2 | tr -d '"' || echo "Unknown")
fi
echo "Target: $current_target"

current_module="Unknown"
if [[ -f "build/project_description.json" ]]; then
  current_module=$(python -c "import json; print(json.load(open('build/project_description.json')).get('project_variables', {}).get('MAIN_MODULE', 'Unknown'))" 2>/dev/null || echo "Unknown")
fi
echo "Module: $current_module"
echo

# Discover modules
files=($(ls -1 main/main_*.[cC] main/main_*.[cC][pP][pP] 2>/dev/null || true))
if ((${#files[@]} == 0)); then
  echo "No main/main_*.c(pp) found" >&2
  exit 1
fi

# Unique module basenames
mods=($(printf "%s\n" "${files[@]##*/}" | sed -E 's/^main_//; s/\.(c|cpp)$//' | sort -u))

# Step 1: Select Module
echo "=== Module Selection ==="
PS3="Select MAIN_MODULE: "
selected_module=""
select m in "${mods[@]}"; do
  [[ -n "${m:-}" ]] || continue
  selected_module="$m"
  echo "Selected module: $selected_module"
  break
done

if [[ -z "$selected_module" ]]; then
  echo "No module selected, exiting." >&2
  exit 1
fi

# Step 2: Select Target
echo
echo "=== Target Selection ==="
targets=("esp32c3" "esp32h2")
PS3="Select ESP32 target: "
selected_target=""
select t in "${targets[@]}"; do
  [[ -n "${t:-}" ]] || continue
  selected_target="$t"
  echo "Selected target: $selected_target"
  break
done

if [[ -z "$selected_target" ]]; then
  echo "No target selected, exiting." >&2
  exit 1
fi

# Step 3: Apply Configuration
echo
echo "=== Applying Configuration ==="
echo "Configuring project with:"
echo "  Module: $selected_module"
echo "  Target: $selected_target"

# Set ESP_MATTER_PATH if module contains "matter"
if [[ "$selected_module" == *"matter"* ]]; then
  export ESP_MATTER_PATH="~/Code/esp-matter"
  # Expand tilde for actual path
  ESP_MATTER_PATH="${ESP_MATTER_PATH/#\~/$HOME}"
  echo "  ESP_MATTER_PATH: $ESP_MATTER_PATH"
fi

# Clean any existing build to avoid conflicts
if [[ -d "build" ]]; then
  echo "Cleaning existing build directory..."
  if ! python "$IDF_PATH/tools/idf.py" fullclean >/dev/null 2>&1; then
    echo "Build directory is corrupted, removing manually..."
    rm -rf build
  fi
fi

# Set target and configure with the selected module
# Must pass MAIN_MODULE from the start to avoid CMakeLists.txt defaulting to all modules
python "$IDF_PATH/tools/idf.py" -D MAIN_MODULE="$selected_module" set-target "$selected_target"

echo
echo "=== Configuration Complete ==="
echo "Project configured successfully!"
echo "  Module: $selected_module"
echo "  Target: $selected_target"

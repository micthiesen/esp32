#![allow(dead_code)]     // Allow unused functions/structs that might be used by other apps
#![warn(unused_imports)]  // Still warn about unused imports - these should be cleaned up
#![warn(unused_variables)] // Still warn about unused variables - these are usually bugs

mod apps;
mod common;

fn main() -> anyhow::Result<()> {
    // Choose which app to run by changing this line:

    apps::blink::run()           // Simple LED blink
    // apps::matter_light::run() // WiFi-enabled LED (Matter-ready)

    // To switch apps, just comment/uncomment the lines above
    // or use cfg attributes for compile-time selection
}

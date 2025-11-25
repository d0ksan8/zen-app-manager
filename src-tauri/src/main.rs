// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Force GDK_BACKEND to x11 to avoid Wayland protocol errors
    std::env::set_var("GDK_BACKEND", "x11");
    // Disable WebKit compositing to fix black screen/GBM errors on some Linux systems
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    zen_app_manager_lib::run()
}

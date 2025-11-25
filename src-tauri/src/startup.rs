use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StartupApp {
    pub id: String,
    pub name: String,
    pub command: String,
    pub enabled: bool,
    pub path: PathBuf,
}

#[cfg(target_os = "linux")]
pub fn get_startup_apps() -> Vec<StartupApp> {
    let mut apps = Vec::new();
    if let Some(config_dir) = dirs::config_dir() {
        let autostart_dir = config_dir.join("autostart");
        if autostart_dir.exists() {
            for entry in WalkDir::new(&autostart_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().extension().map_or(false, |ext| ext == "desktop") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let name = extract_value(&content, "Name").unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                        let raw_command = extract_value(&content, "Exec").unwrap_or_default();
                        let command = raw_command
                            .replace("env GDK_BACKEND=x11 ", "")
                            .replace("env ", "");
                        let hidden = extract_value(&content, "Hidden").map(|v| v.to_lowercase() == "true").unwrap_or(false);
                        let x_gnome_enabled = extract_value(&content, "X-GNOME-Autostart-enabled").map(|v| v.to_lowercase() == "true").unwrap_or(true);
                        
                        // If Hidden is true, it's usually disabled or hidden from menu, but for autostart it implies disabled if it's in autostart dir? 
                        // Actually, in autostart, Hidden=true means it shouldn't start.
                        
                        let enabled = !hidden && x_gnome_enabled;

                        apps.push(StartupApp {
                            id: entry.file_name().to_string_lossy().to_string(),
                            name,
                            command,
                            enabled,
                            path: entry.path().to_path_buf(),
                        });
                    }
                }
            }
        }
    }
    apps
}

#[cfg(target_os = "windows")]
pub fn get_startup_apps() -> Vec<StartupApp> {
    let mut apps = Vec::new();
    if let Some(startup_dir) = dirs::data_dir().map(|d| d.join("Microsoft\\Windows\\Start Menu\\Programs\\Startup")) {
        if startup_dir.exists() {
            for entry in WalkDir::new(&startup_dir).into_iter().filter_map(|e| e.ok()) {
                if entry.path().extension().map_or(false, |ext| ext == "lnk" || ext == "bat" || ext == "cmd" || ext == "exe") {
                    // On Windows, we can't easily parse .lnk files without extra crates, so we'll just use the filename
                    // and assume it's enabled if it exists in the folder.
                    apps.push(StartupApp {
                        id: entry.file_name().to_string_lossy().to_string(),
                        name: entry.file_name().to_string_lossy().replace(".lnk", "").to_string(),
                        command: entry.path().to_string_lossy().to_string(), // Just show path for now
                        enabled: true, // If it's in Startup folder, it's enabled
                        path: entry.path().to_path_buf(),
                    });
                }
            }
        }
    }
    apps
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn get_startup_apps() -> Vec<StartupApp> {
    Vec::new()
}

fn extract_value(content: &str, key: &str) -> Option<String> {
    let key_eq = format!("{}=", key);
    for line in content.lines() {
        if line.starts_with(&key_eq) {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                return Some(parts[1].trim().to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
pub fn toggle_app(path: PathBuf, enable: bool) -> Result<(), String> {
    // To disable, we can set Hidden=true or X-GNOME-Autostart-enabled=false
    // Standard way is Hidden=true
    
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut new_lines = Vec::new();
    let mut hidden_found = false;

    for line in content.lines() {
        if line.starts_with("Hidden=") {
            new_lines.push(format!("Hidden={}", !enable));
            hidden_found = true;
        } else if line.starts_with("X-GNOME-Autostart-enabled=") {
            new_lines.push(format!("X-GNOME-Autostart-enabled={}", enable));
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !hidden_found {
        new_lines.push(format!("Hidden={}", !enable));
    }
    // We don't strictly need to add X-GNOME-Autostart-enabled if not present, but Hidden is standard.

    fs::write(path, new_lines.join("\n")).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn toggle_app(path: PathBuf, enable: bool) -> Result<(), String> {
    // On Windows Startup folder, "disabling" usually means moving it to a "disabled" folder or deleting it.
    // But for simplicity and safety, we might just append ".disabled" to the filename or similar.
    // However, a better approach for a simple manager is to just delete/create.
    // But the user asked for toggle.
    // Let's implement a simple rename strategy: app.lnk -> app.lnk.disabled
    
    let new_path = if enable {
        if path.extension().map_or(false, |e| e == "disabled") {
            path.with_extension("")
        } else {
            return Ok(()); // Already enabled
        }
    } else {
        let mut p = path.clone().into_os_string();
        p.push(".disabled");
        PathBuf::from(p)
    };

    fs::rename(path, new_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn toggle_app(_path: PathBuf, _enable: bool) -> Result<(), String> {
    Err("Not supported on this OS".to_string())
}

#[cfg(target_os = "linux")]
pub fn create_app(name: String, command: String, description: String) -> Result<(), String> {
    if let Some(config_dir) = dirs::config_dir() {
        let autostart_dir = config_dir.join("autostart");
        if !autostart_dir.exists() {
            fs::create_dir_all(&autostart_dir).map_err(|e| e.to_string())?;
        }
        
        // Sanitize filename: replace spaces, slashes, backslashes with hyphens
        let safe_name = name.replace(" ", "-").replace("/", "-").replace("\\", "-").to_lowercase();
        let filename = format!("{}.desktop", safe_name);
        let path = autostart_dir.join(filename);
        
        let content = format!(
            "[Desktop Entry]\nType=Application\nName={}\nExec={}\nComment={}\nHidden=false\nX-GNOME-Autostart-enabled=true\n",
            name, command, description
        );
        
        fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Could not find config directory".to_string())
    }
}

#[cfg(target_os = "windows")]
pub fn create_app(name: String, command: String, _description: String) -> Result<(), String> {
    if let Some(startup_dir) = dirs::data_dir().map(|d| d.join("Microsoft\\Windows\\Start Menu\\Programs\\Startup")) {
         if !startup_dir.exists() {
            fs::create_dir_all(&startup_dir).map_err(|e| e.to_string())?;
        }

        // On Windows, creating a shortcut (.lnk) programmatically is hard without extra libs.
        // We will create a simple .bat file instead.
        let safe_name = name.replace(" ", "-").replace("/", "-").replace("\\", "-").to_lowercase();
        let filename = format!("{}.bat", safe_name);
        let path = startup_dir.join(filename);
        
        // Simple batch file to start the program
        let content = format!("@echo off\nstart \"\" \"{}\"", command);
        
        fs::write(path, content).map_err(|e| e.to_string())?;
        Ok(())
    } else {
         Err("Could not find startup directory".to_string())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn create_app(_name: String, _command: String, _description: String) -> Result<(), String> {
    Err("Not supported on this OS".to_string())
}

pub fn delete_app(path: PathBuf) -> Result<(), String> {
    fs::remove_file(path).map_err(|e| e.to_string())
}

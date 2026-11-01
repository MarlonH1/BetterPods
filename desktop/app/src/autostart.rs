//! "Beim Anmelden starten": an XDG autostart entry on Linux.

#[cfg(target_os = "linux")]
fn entry() -> Option<std::path::PathBuf> {
    Some(dirs::config_dir()?.join("autostart").join("droidpods.desktop"))
}

#[cfg(target_os = "linux")]
pub fn enabled() -> bool {
    entry().is_some_and(|p| p.exists())
}

#[cfg(target_os = "linux")]
pub fn set(on: bool) {
    let Some(path) = entry() else { return };
    let result = if on {
        let exe = std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "droidpods".into());
        let text = format!(
            "[Desktop Entry]\nType=Application\nName=DroidPods\nComment=AirPods companion\nExec={exe} daemon --hidden\nIcon=audio-headphones\nX-GNOME-Autostart-enabled=true\n"
        );
        std::fs::create_dir_all(path.parent().expect("has parent")).and_then(|_| std::fs::write(&path, text))
    } else {
        std::fs::remove_file(&path).or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) })
    };
    if let Err(e) = result {
        log::warn!("autostart: {e}");
    }
}

#[cfg(not(target_os = "linux"))]
pub fn enabled() -> bool {
    false
}

#[cfg(not(target_os = "linux"))]
pub fn set(_on: bool) {}

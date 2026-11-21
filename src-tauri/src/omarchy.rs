use directories::BaseDirs;
use std::collections::BTreeMap;

fn palette(source: &str) -> Option<BTreeMap<&'static str, String>> {
    let table: toml::Table = toml::from_str(source).ok()?;
    let mut colors = BTreeMap::new();
    for key in [
        "background",
        "foreground",
        "accent",
        "lighter_background",
        "selection",
        "green",
        "red",
    ] {
        if let Some(value) = table.get(key).and_then(toml::Value::as_str) {
            if value.len() == 7
                && value.starts_with('#')
                && value[1..].bytes().all(|b| b.is_ascii_hexdigit())
            {
                colors.insert(key, value.to_string());
            }
        }
    }
    for key in ["background", "foreground", "accent"] {
        colors.get(key)?;
    }
    if let Some(mode @ ("light" | "dark")) = table.get("mode").and_then(toml::Value::as_str) {
        colors.insert("mode", mode.to_string());
    }
    Some(colors)
}

#[tauri::command]
pub fn omarchy_palette() -> Option<BTreeMap<&'static str, String>> {
    let dirs = BaseDirs::new()?;
    let path = dirs.state_dir()?.join("omarchy/current/theme/colors.toml");
    palette(&std::fs::read_to_string(path).ok()?)
}

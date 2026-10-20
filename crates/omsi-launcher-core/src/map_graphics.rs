//! Graphics settings kept for certain maps (`~/.openomsi/map_graphics.json`): a map in it is
//! loaded with its own graphics, every other map with the settings of the Graphics page. The
//! page's settings stay as they are: a change of them reaches every map but these.

use anyhow::{anyhow, Result};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::{data_dir, settings_from_text, settings_to_text, GRAPHICS_PROFILE_KEYS};

const FILE: &str = "map_graphics.json";

/// One writer at a time: a save reads the file, changes it and writes it back.
static WRITE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

/// The name a map is kept under: as the launcher and the game write it
/// (`maps/Grundorf/global.cfg`), in lower case, with `/` and without the folders in front of
/// `maps` (a map given by its full path is the same map).
pub fn key(map: &str) -> String {
    let p = map.trim().replace('\\', "/");
    let p = p.trim_start_matches("./");
    let lower = p.to_ascii_lowercase();
    let from = if lower.starts_with('/') || lower.get(1..2) == Some(":") { lower.rfind("/maps/").map(|i| i + 1).unwrap_or(0) } else { 0 };
    lower[from..].to_string()
}

fn read_in(dir: &Path) -> BTreeMap<String, Value> {
    std::fs::read_to_string(path_in(dir)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn write_in(dir: &Path, all: &BTreeMap<String, Value>) -> Result<()> {
    std::fs::write(path_in(dir), serde_json::to_string_pretty(all)?)?;
    Ok(())
}

/// What `settings` holds of the graphics a map may have of its own: the keys of a graphics
/// profile.
fn graphics_of(settings: &Value) -> Map<String, Value> {
    GRAPHICS_PROFILE_KEYS.iter().filter_map(|k| settings.get(*k).map(|v| (k.to_string(), v.clone()))).collect()
}

fn save_in(dir: &Path, map: &str, settings: &Value) -> Result<()> {
    let name = key(map);
    if name.is_empty() {
        return Err(anyhow!("Choose a map first."));
    }
    let own = graphics_of(settings);
    if own.is_empty() {
        return Err(anyhow!("There are no graphics settings to keep."));
    }
    let _one = WRITE.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = read_in(dir);
    all.insert(name, Value::Object(own));
    write_in(dir, &all)
}

fn delete_in(dir: &Path, map: &str) -> Result<()> {
    let _one = WRITE.lock().unwrap_or_else(|e| e.into_inner());
    let mut all = read_in(dir);
    if all.remove(&key(map)).is_some() {
        write_in(dir, &all)?;
    }
    Ok(())
}

/// The graphics kept for each map, by `key`.
pub fn all() -> BTreeMap<String, Value> {
    read_in(&data_dir())
}

/// The graphics kept for `map`, if it has any of its own.
pub fn of(map: &str) -> Option<Value> {
    all().remove(&key(map)).filter(Value::is_object)
}

/// Keep the graphics of `settings` for `map` (what was kept for it is replaced).
pub fn save(map: &str, settings: &Value) -> Result<()> {
    save_in(&data_dir(), map, settings)
}

/// Let `map` have the settings of the Graphics page again.
pub fn delete(map: &str) -> Result<()> {
    delete_in(&data_dir(), map)
}

/// The settings of the page with the graphics kept for the map put over them.
pub fn put_over(own: &Value, settings: &mut Value) {
    if let Some(m) = own.as_object() {
        for k in GRAPHICS_PROFILE_KEYS {
            if let Some(v) = m.get(k) {
                settings[k] = v.clone();
            }
        }
    }
}

/// The text of a `settings.cfg` with the graphics kept for `map` put over it - the game's
/// settings for that map. None when the map keeps none of its own (the text is the game's
/// as it is).
pub fn text_for(map: &str, text: &str) -> Option<String> {
    text_for_in(&data_dir(), map, text)
}

fn text_for_in(dir: &Path, map: &str, text: &str) -> Option<String> {
    let own = read_in(dir).remove(&key(map)).filter(Value::is_object)?;
    Some(text_over(&own, text))
}

/// `text` (a `settings.cfg`) with the kept graphics `own` over it: the lines of those settings
/// are written after the file's own, and the last line of a setting is the one the game
/// reads. Everything else of the file - the volume, a setting of a newer game - is left as it
/// is.
pub fn text_over(own: &Value, text: &str) -> String {
    let mut v = settings_from_text(Some(text));
    put_over(own, &mut v);
    let written = settings_to_text(&v, Some(text));
    let mut out = text.trim_end().to_string();
    out.push('\n');
    for line in written.lines() {
        if let Some((k, _)) = line.split_once('=') {
            if own.get(k.trim()).is_some() && GRAPHICS_PROFILE_KEYS.contains(&k.trim()) {
                out.push_str(line.trim_end());
                out.push('\n');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("openomsi-map-graphics-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_map_has_one_name_however_it_is_written() {
        let a = key("maps/Grundorf/global.cfg");
        assert_eq!(a, "maps/grundorf/global.cfg");
        assert_eq!(key("maps\\Grundorf\\global.cfg"), a);
        assert_eq!(key("./maps/Grundorf/global.cfg"), a);
        assert_eq!(key("  maps/GRUNDORF/global.cfg "), a);
        assert_eq!(key("C:\\Games\\OMSI 2\\maps\\Grundorf\\global.cfg"), a);
        assert_eq!(key("/home/me/OMSI 2/Maps/Grundorf/global.cfg"), a);
        assert_eq!(key(""), "");
    }

    #[test]
    fn what_is_kept_is_the_graphics_only() {
        let dir = scratch("keep");
        let settings = json!({ "msaa": 2, "shadow_size": 1024, "volume": 0.2, "language": "GER", "fullscreen": true, "max_obj_dist": "750" });
        save_in(&dir, "maps/Grundorf/global.cfg", &settings).unwrap();
        let kept = read_in(&dir).remove("maps/grundorf/global.cfg").unwrap();
        assert_eq!(kept, json!({ "msaa": 2, "shadow_size": 1024, "max_obj_dist": "750" }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_map_is_changed_and_given_back_without_touching_the_others() {
        let dir = scratch("change");
        save_in(&dir, "maps/A/global.cfg", &json!({ "msaa": 2 })).unwrap();
        save_in(&dir, "maps/B/global.cfg", &json!({ "msaa": 8 })).unwrap();
        // changed again later: replaced, not added to
        save_in(&dir, "maps\\A\\global.cfg", &json!({ "shadow_size": 4096 })).unwrap();
        let all = read_in(&dir);
        assert_eq!(all.len(), 2);
        assert_eq!(all["maps/a/global.cfg"], json!({ "shadow_size": 4096 }));
        assert_eq!(all["maps/b/global.cfg"], json!({ "msaa": 8 }));
        delete_in(&dir, "maps/A/global.cfg").unwrap();
        delete_in(&dir, "maps/never-kept/global.cfg").unwrap();
        let all = read_in(&dir);
        assert_eq!(all.len(), 1);
        assert!(all.contains_key("maps/b/global.cfg"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_to_keep_and_no_map_are_told() {
        let dir = scratch("empty");
        assert!(save_in(&dir, "", &json!({ "msaa": 2 })).is_err());
        assert!(save_in(&dir, "maps/A/global.cfg", &json!({ "volume": 1 })).is_err());
        assert!(read_in(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_file_is_no_graphics_for_any_map() {
        let dir = scratch("broken");
        std::fs::write(path_in(&dir), "{ not json").unwrap();
        assert!(read_in(&dir).is_empty());
        assert!(text_for_in(&dir, "maps/A/global.cfg", "msaa=4\n").is_none());
        // and the next save writes it again, whole
        save_in(&dir, "maps/A/global.cfg", &json!({ "msaa": 2 })).unwrap();
        assert_eq!(read_in(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_game_reads_the_kept_graphics_for_that_map_and_the_page_for_every_other() {
        let dir = scratch("text");
        let page = "msaa=4\nssao=1\nshadow_size=2048\nmax_obj_dist=1500\nvolume=0.6\nsome_future_key=7\n";
        let own = json!({ "msaa": 2, "ssao": false, "shadow_size": 1024, "max_obj_dist": "750", "volume": 0.1 });
        save_in(&dir, "maps/Grundorf/global.cfg", &own).unwrap();
        let text = text_for_in(&dir, "maps/grundorf/global.cfg", page).unwrap();
        let v = settings_from_text(Some(&text));
        assert_eq!(v["msaa"], json!(2));
        assert_eq!(v["ssao"], json!(false));
        assert_eq!(v["shadow_size"], json!(1024));
        assert_eq!(v["max_obj_dist"].to_string().trim_matches('"'), "750");
        // (what is not graphics - the volume, a key of a newer game - stays the page's)
        assert!((v["volume"].as_f64().unwrap() - 0.6).abs() < 1e-6);
        assert!(text.contains("some_future_key=7"));
        // (the page's own lines stay, the kept ones come after them)
        assert!(text.starts_with(page.trim_end()));
        assert!(text.rfind("msaa=2").unwrap() > text.find("msaa=4").unwrap());
        // another map: nothing of its own, the page's text as it is
        assert!(text_for_in(&dir, "maps/Other/global.cfg", page).is_none());
        // the page's change reaches it as before; the map with its own graphics keeps them
        let later = page.replace("msaa=4", "msaa=8");
        assert_eq!(settings_from_text(Some(&text_for_in(&dir, "maps/grundorf/global.cfg", &later).unwrap()))["msaa"], json!(2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_graphics_setting_of_a_profile_reaches_the_game() {
        // a setting the settings text does not hold would be kept for a map and never read
        let others: [(&str, Value); 31] = [
            ("graphics", json!("enhanced")),
            ("msaa", json!(2)),
            ("render_scale", json!("0.75")),
            ("anisotropy", json!(2)),
            ("shadow_size", json!(1024)),
            ("ssao", json!(false)),
            ("shadows", json!(false)),
            ("shadow_casters", json!("omsi")),
            ("detail_textures", json!(false)),
            ("night_brightness", json!(1.5)),
            ("led_glow", json!(3)),
            ("led_mips", json!(0.5)),
            ("shadow_blobs", json!(false)),
            ("reflections", json!(0.5)),
            ("clouds", json!(false)),
            ("cloud_quality", json!("low")),
            ("rain_quality", json!("low")),
            ("puddles", json!("off")),
            ("windy_trees", json!(false)),
            ("vsync", json!(false)),
            ("max_fps", json!(60)),
            ("view_distance", json!("900")),
            ("max_obj_dist", json!("750")),
            ("min_obj_size", json!(0.02)),
            ("mirror_size", json!(512)),
            ("mirror_refresh", json!("eco")),
            ("mirror_quality", json!("enhanced")),
            ("mirror_hud_every_frame", json!(true)),
            ("video_memory", json!(4000)),
            ("texture_compression", json!(false)),
            ("gpu_texture_compression", json!(false)),
        ];
        // (all of them, so that a key added to the profiles is added here)
        assert_eq!(others.iter().map(|(k, _)| *k).collect::<Vec<_>>(), GRAPHICS_PROFILE_KEYS.to_vec());
        let mut lost = Vec::new();
        for (k, v) in &others {
            let mut page = settings_from_text(Some(""));
            page[*k] = v.clone();
            let back = settings_from_text(Some(&settings_to_text(&page, None)));
            if back.get(*k) != Some(v) && !same_number(back.get(*k), Some(v)) {
                lost.push(format!("{k}: {v} came back as {:?}", back.get(*k)));
            }
        }
        assert!(lost.is_empty(), "kept for a map but not written to the settings: {lost:?}");
    }

    fn same_number(a: Option<&Value>, b: Option<&Value>) -> bool {
        let n = |v: Option<&Value>| v.and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())));
        matches!((n(a), n(b)), (Some(x), Some(y)) if (x - y).abs() < 1e-6)
    }
}

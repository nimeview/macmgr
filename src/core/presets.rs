use anyhow::{anyhow, Result};

use crate::core::manifest::Manifest;
use crate::util::shell::run_status;

pub fn list(manifest: &Manifest) -> Vec<(String, String)> {
    manifest
        .presets
        .iter()
        .map(|p| {
            (
                p.id.clone(),
                format!(
                    "{} — padding={} gap={} opacity={:.2}",
                    p.label, p.padding, p.gap, p.opacity
                ),
            )
        })
        .collect()
}

pub fn apply(manifest: &Manifest, id: &str) -> Result<()> {
    let preset = manifest
        .find_preset(id)
        .ok_or_else(|| anyhow!("unknown preset `{id}`"))?;

    // Apply via yabai config commands (runtime); does not rewrite yabairc
    let pad = preset.padding.to_string();
    let gap = preset.gap.to_string();
    let opacity = format!("{:.2}", preset.opacity);

    run_status("yabai", &["-m", "config", "top_padding", &pad])?;
    run_status("yabai", &["-m", "config", "bottom_padding", &pad])?;
    run_status("yabai", &["-m", "config", "left_padding", &pad])?;
    run_status("yabai", &["-m", "config", "right_padding", &pad])?;
    run_status("yabai", &["-m", "config", "window_gap", &gap])?;
    run_status("yabai", &["-m", "config", "normal_window_opacity", &opacity])?;
    Ok(())
}

//! Audio devices, as `pactl -f json list sinks` and `... sources` report them. PipeWire and
//! PulseAudio both answer it, so the Sound page needs no audio library.

use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// The name `pactl` takes to select it.
    pub name: String,
    /// What the user reads.
    pub description: String,
    /// The loudest channel's volume, in percent.
    pub volume: u32,
    pub muted: bool,
}

#[derive(Deserialize)]
struct Channel {
    #[serde(default)]
    value_percent: String,
}

#[derive(Deserialize)]
struct Raw {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    mute: bool,
    #[serde(default)]
    volume: std::collections::HashMap<String, Channel>,
    /// Set on a source that only listens to an output.
    #[serde(default)]
    monitor_of_sink: Option<serde_json::Value>,
}

fn percent(text: &str) -> Option<u32> {
    text.trim().strip_suffix('%')?.trim().parse().ok()
}

/// The devices in `json`, without the monitors of outputs.
pub fn parse_devices(json: &str) -> Result<Vec<Device>, serde_json::Error> {
    let raw: Vec<Raw> = serde_json::from_str(json)?;
    Ok(raw
        .into_iter()
        .filter(|d| d.monitor_of_sink.as_ref().is_none_or(|v| v.is_null()))
        .map(|d| Device {
            volume: d
                .volume
                .values()
                .filter_map(|c| percent(&c.value_percent))
                .max()
                .unwrap_or(0),
            description: if d.description.is_empty() {
                d.name.clone()
            } else {
                d.description
            },
            name: d.name,
            muted: d.mute,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINKS: &str = r#"[
      {"name":"alsa_output.pci","description":"Built-in Audio","mute":false,
       "volume":{"front-left":{"value":39000,"value_percent":"60%"},"front-right":{"value":42000,"value_percent":"64%"}},
       "monitor_of_sink":null},
      {"name":"alsa_output.monitor","description":"Monitor of Built-in Audio","mute":true,"volume":{},
       "monitor_of_sink":"alsa_output.pci"}
    ]"#;

    #[test]
    fn reads_the_loudest_channel_and_the_mute_flag() {
        let devices = parse_devices(SINKS).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "alsa_output.pci");
        assert_eq!(devices[0].description, "Built-in Audio");
        assert_eq!(devices[0].volume, 64);
        assert!(!devices[0].muted);
    }

    #[test]
    fn a_device_without_a_description_is_shown_by_name() {
        let devices = parse_devices(r#"[{"name":"x","volume":{}}]"#).unwrap();
        assert_eq!(devices[0].description, "x");
        assert_eq!(devices[0].volume, 0);
    }

    #[test]
    fn rejects_text_that_is_not_the_expected_list() {
        assert!(parse_devices("not json").is_err());
        assert!(parse_devices("{}").is_err());
    }

    #[test]
    fn reads_percentages() {
        assert_eq!(percent("100%"), Some(100));
        assert_eq!(percent(" 7 %"), Some(7));
        assert_eq!(percent("loud"), None);
    }
}

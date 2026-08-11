//! ASUS hardware control widget.

use vibepanel_core::config::WidgetEntry;

use crate::services::control::AsusControl;
use crate::services::icons::IconHandle;
use crate::styles::widget;
use crate::widgets::asus_control_popover::build_asus_control_popover;
use crate::widgets::{BaseWidget, WidgetConfig, warn_unknown_options};

const DEFAULT_ICON: &str = "system-monitor-symbolic";
const DEFAULT_ASUSCTL_COMMAND: &str = "asusctl";
const DEFAULT_SUPERGFXCTL_COMMAND: &str = "supergfxctl";
const DEFAULT_NIRI_COMMAND: &str = "niri";

#[derive(Debug, Clone)]
pub struct AsusControlConfig {
    pub icon: String,
    pub asusctl_command: String,
    pub supergfxctl_command: String,
    pub niri_command: String,
}

impl WidgetConfig for AsusControlConfig {
    fn from_entry(entry: &WidgetEntry) -> Self {
        warn_unknown_options(
            "asus_control",
            entry,
            &[
                "icon",
                "asusctl_command",
                "supergfxctl_command",
                "niri_command",
            ],
        );

        Self {
            icon: string_option(entry, "icon", DEFAULT_ICON),
            asusctl_command: string_option(entry, "asusctl_command", DEFAULT_ASUSCTL_COMMAND),
            supergfxctl_command: string_option(
                entry,
                "supergfxctl_command",
                DEFAULT_SUPERGFXCTL_COMMAND,
            ),
            niri_command: string_option(entry, "niri_command", DEFAULT_NIRI_COMMAND),
        }
    }
}

impl Default for AsusControlConfig {
    fn default() -> Self {
        Self {
            icon: DEFAULT_ICON.to_string(),
            asusctl_command: DEFAULT_ASUSCTL_COMMAND.to_string(),
            supergfxctl_command: DEFAULT_SUPERGFXCTL_COMMAND.to_string(),
            niri_command: DEFAULT_NIRI_COMMAND.to_string(),
        }
    }
}

pub struct AsusControlWidget {
    base: BaseWidget,
    _icon_handle: IconHandle,
}

impl AsusControlWidget {
    pub fn new(config: AsusControlConfig) -> Self {
        let base = BaseWidget::new(&[widget::ASUS_CONTROL]);
        base.set_tooltip("ASUS Control");
        let icon_handle = base.add_icon(&config.icon, &[widget::ASUS_CONTROL_ICON]);

        let control = AsusControl::new(
            config.asusctl_command,
            config.supergfxctl_command,
            config.niri_command,
        );
        base.create_menu(move || build_asus_control_popover(control.clone()));

        Self {
            base,
            _icon_handle: icon_handle,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        self.base.widget()
    }

    pub(crate) fn edge_interaction(&self) -> Option<crate::widgets::EdgeInteraction> {
        self.base.edge_interaction()
    }
}

fn string_option(entry: &WidgetEntry, key: &str, default: &str) -> String {
    entry
        .options
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(default)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_and_icon_overrides() {
        let mut entry = WidgetEntry::new("asus_control");
        entry.options.insert(
            "icon".to_string(),
            toml::Value::String("configure-symbolic".to_string()),
        );
        entry.options.insert(
            "asusctl_command".to_string(),
            toml::Value::String("/opt/bin/asusctl".to_string()),
        );

        let config = AsusControlConfig::from_entry(&entry);
        assert_eq!(config.icon, "configure-symbolic");
        assert_eq!(config.asusctl_command, "/opt/bin/asusctl");
        assert_eq!(config.supergfxctl_command, "supergfxctl");
        assert_eq!(config.niri_command, "niri");
    }
}

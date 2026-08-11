//! systemd-backed VPN control widget.

use vibepanel_core::config::WidgetEntry;

use crate::services::control::SystemdControl;
use crate::services::icons::IconHandle;
use crate::styles::widget;
use crate::widgets::vpn_control_popover::{VpnServiceTarget, build_vpn_control_popover};
use crate::widgets::{BaseWidget, WidgetConfig, warn_unknown_options};

const DEFAULT_ICON: &str = "network-vpn";
const DEFAULT_WIREGUARD_SERVICE: &str = "wg-quick@wg-uk.service";
const DEFAULT_SING_BOX_SERVICE: &str = "sing-box.service";
const DEFAULT_SUDO_COMMAND: &str = "sudo";
const DEFAULT_SYSTEMCTL_COMMAND: &str = "systemctl";

#[derive(Debug, Clone)]
pub struct VpnControlConfig {
    pub icon: String,
    pub wireguard_service: String,
    pub sing_box_service: String,
    pub sudo_command: String,
    pub systemctl_command: String,
}

impl WidgetConfig for VpnControlConfig {
    fn from_entry(entry: &WidgetEntry) -> Self {
        warn_unknown_options(
            "vpn_control",
            entry,
            &[
                "icon",
                "wireguard_service",
                "sing_box_service",
                "sudo_command",
                "systemctl_command",
            ],
        );

        Self {
            icon: string_option(entry, "icon", DEFAULT_ICON),
            wireguard_service: string_option(entry, "wireguard_service", DEFAULT_WIREGUARD_SERVICE),
            sing_box_service: string_option(entry, "sing_box_service", DEFAULT_SING_BOX_SERVICE),
            sudo_command: string_option(entry, "sudo_command", DEFAULT_SUDO_COMMAND),
            systemctl_command: string_option(entry, "systemctl_command", DEFAULT_SYSTEMCTL_COMMAND),
        }
    }
}

impl Default for VpnControlConfig {
    fn default() -> Self {
        Self {
            icon: DEFAULT_ICON.to_string(),
            wireguard_service: DEFAULT_WIREGUARD_SERVICE.to_string(),
            sing_box_service: DEFAULT_SING_BOX_SERVICE.to_string(),
            sudo_command: DEFAULT_SUDO_COMMAND.to_string(),
            systemctl_command: DEFAULT_SYSTEMCTL_COMMAND.to_string(),
        }
    }
}

pub struct VpnControlWidget {
    base: BaseWidget,
    _icon_handle: IconHandle,
}

impl VpnControlWidget {
    pub fn new(config: VpnControlConfig) -> Self {
        let base = BaseWidget::new(&[widget::VPN_CONTROL]);
        base.set_tooltip("VPN Control");
        let icon_handle = base.add_icon(&config.icon, &[widget::VPN_CONTROL_ICON]);

        let control = SystemdControl::new(config.sudo_command, config.systemctl_command);
        let targets = [
            VpnServiceTarget::new("WireGuard UK", "network-vpn", config.wireguard_service),
            VpnServiceTarget::new(
                "sing-box",
                "network-transmit-receive-symbolic",
                config.sing_box_service,
            ),
        ];
        base.create_menu(move || build_vpn_control_popover(control.clone(), targets.clone()));

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
    fn parses_service_overrides() {
        let mut entry = WidgetEntry::new("vpn_control");
        entry.options.insert(
            "wireguard_service".to_string(),
            toml::Value::String("wg-quick@home.service".to_string()),
        );

        let config = VpnControlConfig::from_entry(&entry);
        assert_eq!(config.wireguard_service, "wg-quick@home.service");
        assert_eq!(config.sing_box_service, "sing-box.service");
        assert_eq!(config.sudo_command, "sudo");
    }
}

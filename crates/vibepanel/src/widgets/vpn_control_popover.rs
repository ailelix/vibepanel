//! VPN service control popover content and asynchronous actions.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Orientation, Widget};
use tracing::warn;

use crate::services::control::SystemdControl;
use crate::services::icons::IconHandle;
use crate::styles::control as style;
use crate::widgets::control_panel::{action_button, set_active, set_error};

#[derive(Debug, Clone)]
pub struct VpnServiceTarget {
    label: &'static str,
    icon: &'static str,
    unit: String,
}

impl VpnServiceTarget {
    pub fn new(label: &'static str, icon: &'static str, unit: String) -> Self {
        Self { label, icon, unit }
    }
}

pub fn build_vpn_control_popover(
    control: SystemdControl,
    targets: [VpnServiceTarget; 2],
) -> Widget {
    let container = GtkBox::new(Orientation::Horizontal, 6);
    container.add_css_class(style::PANEL);
    container.add_css_class(style::VPN_PANEL);
    container.add_css_class(style::ROW);
    container.set_homogeneous(true);

    for target in targets {
        let tooltip = format!("Toggle {} ({})", target.label, target.unit);
        let action_button = action_button(target.label, target.icon, &tooltip);
        let (button, icon_handle) = action_button.into_parts();
        container.append(&button);
        connect_service_button(&button, icon_handle, control.clone(), target);
    }

    container.upcast()
}

fn connect_service_button(
    button: &Button,
    icon_handle: IconHandle,
    control: SystemdControl,
    target: VpnServiceTarget,
) {
    let active = Rc::new(Cell::new(None));

    {
        let button_weak = button.downgrade();
        let control = control.clone();
        let target = target.clone();
        let active = Rc::clone(&active);
        button.connect_clicked(move |_| {
            let _keep_icon_alive = &icon_handle;
            let Some(button) = button_weak.upgrade() else {
                return;
            };
            let requested_active = !active.get().unwrap_or(false);
            button.set_sensitive(false);

            let button = button.clone();
            let active = Rc::clone(&active);
            let unit = target.unit.clone();
            let label = target.label;
            let control = control.clone();
            let unit_for_command = unit.clone();
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || {
                    control.set_active(&unit_for_command, requested_active)
                })
                .await;

                match result {
                    Ok(Ok(actual_active)) => {
                        active.set(Some(actual_active));
                        apply_service_state(&button, label, actual_active);
                    }
                    Ok(Err(error)) => {
                        warn!(%unit, %error, "Failed to toggle VPN service");
                        set_error(&button, &format!("{unit}: {error}"));
                    }
                    Err(_) => {
                        let error = "Service control worker panicked".to_string();
                        warn!(%unit, %error);
                        set_error(&button, &error);
                    }
                }
                button.set_sensitive(true);
            });
        });
    }

    button.set_sensitive(false);
    let button = button.clone();
    let unit = target.unit.clone();
    let label = target.label;
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || control.is_active(&unit)).await;
        match result {
            Ok(Ok(is_active)) => {
                active.set(Some(is_active));
                apply_service_state(&button, label, is_active);
            }
            Ok(Err(error)) => {
                warn!(unit = %target.unit, %error, "Failed to read VPN service state");
                set_error(&button, &format!("{}: {error}", target.unit));
            }
            Err(_) => {
                let error = "Service state worker panicked".to_string();
                warn!(unit = %target.unit, %error);
                set_error(&button, &error);
            }
        }
        button.set_sensitive(true);
    });
}

fn apply_service_state(button: &Button, label: &str, active: bool) {
    set_active(button, active);
    button.set_tooltip_text(Some(&format!(
        "{label}: {}",
        if active { "active" } else { "inactive" }
    )));
}

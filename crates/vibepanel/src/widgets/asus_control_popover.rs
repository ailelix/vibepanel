//! ASUS control popover content and asynchronous actions.

use std::cell::Cell;
use std::rc::Rc;

use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Orientation, Widget};
use tracing::{debug, warn};

use crate::services::compositor::CompositorManager;
use crate::services::control::{AsusControl, FanProfile, GraphicsMode};
use crate::styles::control as style;
use crate::widgets::control_panel::{ActionButton, action_button, set_active, set_error};

type FanButtons = Rc<Vec<(glib::WeakRef<Button>, FanProfile)>>;
type GraphicsButtons = Rc<Vec<(glib::WeakRef<Button>, GraphicsMode)>>;

pub fn build_asus_control_popover(control: AsusControl) -> Widget {
    let container = GtkBox::new(Orientation::Vertical, 10);
    container.add_css_class(style::PANEL);
    container.add_css_class(style::ASUS_PANEL);

    let fan_actions: Vec<(ActionButton, FanProfile)> = FanProfile::ALL
        .into_iter()
        .map(|profile| {
            let icon = match profile {
                FanProfile::Quiet => "weather-wind",
                FanProfile::Balanced => "system-monitor-symbolic",
                FanProfile::Performance => "cpu-symbolic",
            };
            let tooltip = format!("Use the {} ASUS fan profile", profile.label());
            (action_button(profile.label(), icon, &tooltip), profile)
        })
        .collect();
    let fan_row = homogeneous_row(fan_actions.iter().map(|(button, _)| button.widget()));
    container.append(&fan_row);
    let fan_buttons = Rc::new(
        fan_actions
            .iter()
            .map(|(button, profile)| (button.widget().downgrade(), *profile))
            .collect(),
    );

    let graphics_actions: Vec<(ActionButton, GraphicsMode)> = GraphicsMode::ALL
        .into_iter()
        .map(|mode| {
            let (label, icon) = match mode {
                GraphicsMode::Integrated => ("Integrated", "video-display-symbolic"),
                GraphicsMode::Hybrid => ("Hybrid", "computer-symbolic"),
            };
            let tooltip = format!("Switch to {label} graphics and confirm logout");
            (action_button(label, icon, &tooltip), mode)
        })
        .collect();
    let graphics_row = homogeneous_row(graphics_actions.iter().map(|(button, _)| button.widget()));
    container.append(&graphics_row);
    let graphics_buttons = Rc::new(
        graphics_actions
            .iter()
            .map(|(button, mode)| (button.widget().downgrade(), *mode))
            .collect(),
    );

    let current_fan = Rc::new(Cell::new(None));
    connect_fan_actions(&control, fan_actions, &fan_buttons, &current_fan);
    refresh_fan_profile(
        control.clone(),
        Rc::clone(&fan_buttons),
        Rc::clone(&current_fan),
    );

    let current_graphics = Rc::new(Cell::new(None));
    let requested_graphics = Rc::new(Cell::new(None));
    connect_graphics_actions(
        &control,
        graphics_actions,
        &graphics_buttons,
        &current_graphics,
        &requested_graphics,
    );
    refresh_graphics_mode(
        control,
        Rc::clone(&graphics_buttons),
        Rc::clone(&current_graphics),
        Rc::clone(&requested_graphics),
    );

    container.upcast()
}

fn homogeneous_row<'a>(buttons: impl Iterator<Item = &'a Button>) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 6);
    row.add_css_class(style::ROW);
    row.set_homogeneous(true);
    for button in buttons {
        row.append(button);
    }
    row
}

fn connect_fan_actions(
    control: &AsusControl,
    actions: Vec<(ActionButton, FanProfile)>,
    buttons: &FanButtons,
    current: &Rc<Cell<Option<FanProfile>>>,
) {
    for (button, profile) in actions {
        let (button, icon_handle) = button.into_parts();
        let control = control.clone();
        let buttons = Rc::clone(buttons);
        let current = Rc::clone(current);

        button.connect_clicked(move |_| {
            let _keep_icon_alive = &icon_handle;
            if current.get() == Some(profile) {
                return;
            }

            set_sensitive(&buttons, false);
            let control = control.clone();
            let buttons = Rc::clone(&buttons);
            let current = Rc::clone(&current);
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || control.set_fan_profile(profile)).await;
                match result {
                    Ok(Ok(())) => {
                        current.set(Some(profile));
                        apply_fan_state(&buttons, Some(profile));
                        debug!(profile = profile.label(), "ASUS fan profile changed");
                    }
                    Ok(Err(error)) => {
                        warn!(%error, "Failed to change ASUS fan profile");
                        set_group_error(&buttons, &error);
                    }
                    Err(_) => {
                        let error = "ASUS profile worker panicked".to_string();
                        warn!(%error);
                        set_group_error(&buttons, &error);
                    }
                }
                set_sensitive(&buttons, true);
            });
        });
    }
}

fn refresh_fan_profile(
    control: AsusControl,
    buttons: FanButtons,
    current: Rc<Cell<Option<FanProfile>>>,
) {
    set_sensitive(&buttons, false);
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(move || control.fan_profile()).await;
        match result {
            Ok(Ok(profile)) => {
                current.set(Some(profile));
                apply_fan_state(&buttons, Some(profile));
            }
            Ok(Err(error)) => {
                warn!(%error, "Failed to read ASUS fan profile");
                set_group_error(&buttons, &error);
            }
            Err(_) => {
                let error = "ASUS profile worker panicked".to_string();
                warn!(%error);
                set_group_error(&buttons, &error);
            }
        }
        set_sensitive(&buttons, true);
    });
}

fn connect_graphics_actions(
    control: &AsusControl,
    actions: Vec<(ActionButton, GraphicsMode)>,
    buttons: &GraphicsButtons,
    current: &Rc<Cell<Option<GraphicsMode>>>,
    requested: &Rc<Cell<Option<GraphicsMode>>>,
) {
    for (button, mode) in actions {
        let (button, icon_handle) = button.into_parts();
        let control = control.clone();
        let buttons = Rc::clone(buttons);
        let current = Rc::clone(current);
        let requested = Rc::clone(requested);

        button.connect_clicked(move |clicked_button| {
            let _keep_icon_alive = &icon_handle;
            let requested_mode = requested.get();
            if current.get() == Some(mode) && requested_mode.is_none() {
                return;
            }

            if CompositorManager::global().backend_name() != "Niri" {
                let message = "Automatic graphics switching is supported only on Niri";
                set_error(clicked_button, message);
                warn!("{message}");
                return;
            }

            set_sensitive(&buttons, false);
            let control = control.clone();
            let buttons = Rc::clone(&buttons);
            let current = Rc::clone(&current);
            let requested = Rc::clone(&requested);
            glib::spawn_future_local(async move {
                if requested_mode == Some(mode) {
                    request_niri_logout(control, Rc::clone(&buttons), mode).await;
                    set_sensitive(&buttons, true);
                    return;
                }

                let mode_control = control.clone();
                let result = gio::spawn_blocking(move || {
                    mode_control.set_graphics_mode(mode)?;
                    mode_control.pending_graphics_mode()
                })
                .await;
                match result {
                    Ok(Ok(pending_mode)) => {
                        requested.set(pending_mode);
                        // Do not alter the highlighted mode. If the user cancels
                        // Niri's confirmation, the current mode remains correct;
                        // if they accept, vibepanel restarts after the new session.
                        apply_graphics_state(&buttons, current.get());
                        if let Some(pending_mode) = pending_mode {
                            request_niri_logout(control, Rc::clone(&buttons), pending_mode).await;
                        } else {
                            debug!(mode = mode.label(), "Cleared pending graphics mode");
                        }
                    }
                    Ok(Err(error)) => {
                        requested.set(None);
                        warn!(%error, "Failed to request graphics mode");
                        set_group_error(&buttons, &error);
                    }
                    Err(_) => {
                        requested.set(None);
                        let error = "Graphics mode worker panicked".to_string();
                        warn!(%error);
                        set_group_error(&buttons, &error);
                    }
                }
                set_sensitive(&buttons, true);
            });
        });
    }
}

fn refresh_graphics_mode(
    control: AsusControl,
    buttons: GraphicsButtons,
    current: Rc<Cell<Option<GraphicsMode>>>,
    requested: Rc<Cell<Option<GraphicsMode>>>,
) {
    set_sensitive(&buttons, false);
    glib::spawn_future_local(async move {
        let result =
            gio::spawn_blocking(move || (control.graphics_mode(), control.pending_graphics_mode()))
                .await;
        match result {
            Ok((current_result, requested_result)) => {
                match current_result {
                    Ok(mode) => {
                        current.set(Some(mode));
                        apply_graphics_state(&buttons, Some(mode));
                    }
                    Err(error) => {
                        warn!(%error, "Failed to read graphics mode");
                        set_group_error(&buttons, &error);
                    }
                }

                match requested_result {
                    Ok(mode) => requested.set(mode),
                    Err(error) => {
                        requested.set(None);
                        warn!(%error, "Failed to read pending graphics mode");
                        set_group_error(&buttons, &error);
                    }
                }
            }
            Err(_) => {
                let error = "Graphics mode worker panicked".to_string();
                warn!(%error);
                set_group_error(&buttons, &error);
            }
        }
        set_sensitive(&buttons, true);
    });
}

async fn request_niri_logout(control: AsusControl, buttons: GraphicsButtons, mode: GraphicsMode) {
    let logout_result = gio::spawn_blocking(move || control.request_logout_confirmation()).await;
    match logout_result {
        Ok(Ok(())) => {
            debug!(
                mode = mode.label(),
                "Requested graphics mode and Niri logout"
            );
        }
        Ok(Err(error)) => {
            warn!(%error, "Failed to open Niri logout confirmation");
            set_group_error(&buttons, &error);
        }
        Err(_) => {
            let error = "Niri logout worker panicked";
            warn!(%error);
            set_group_error(&buttons, error);
        }
    }
}

fn apply_fan_state(buttons: &FanButtons, current: Option<FanProfile>) {
    for (button, profile) in buttons.iter() {
        if let Some(button) = button.upgrade() {
            set_active(&button, current == Some(*profile));
            button.set_tooltip_text(Some(&format!(
                "Use the {} ASUS fan profile",
                profile.label()
            )));
        }
    }
}

fn apply_graphics_state(buttons: &GraphicsButtons, current: Option<GraphicsMode>) {
    for (button, mode) in buttons.iter() {
        if let Some(button) = button.upgrade() {
            set_active(&button, current == Some(*mode));
            button.set_tooltip_text(Some(&format!(
                "Switch to {} graphics and confirm logout",
                mode.label()
            )));
        }
    }
}

fn set_sensitive<T>(buttons: &Rc<Vec<(glib::WeakRef<Button>, T)>>, sensitive: bool) {
    for (button, _) in buttons.iter() {
        if let Some(button) = button.upgrade() {
            button.set_sensitive(sensitive);
        }
    }
}

fn set_group_error<T>(buttons: &Rc<Vec<(glib::WeakRef<Button>, T)>>, error: &str) {
    for (button, _) in buttons.iter() {
        if let Some(button) = button.upgrade() {
            set_error(&button, error);
        }
    }
}

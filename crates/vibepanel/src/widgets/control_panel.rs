//! Shared presentation helpers for compact command-backed control panels.

use gtk4::prelude::*;
use gtk4::{Align, Box as GtkBox, Button, Label, Orientation};

use crate::services::icons::{IconHandle, IconsService};
use crate::styles::{button, control, state};
use crate::widgets::base::vp_button;

pub struct ActionButton {
    button: Button,
    icon_handle: IconHandle,
}

impl ActionButton {
    pub fn widget(&self) -> &Button {
        &self.button
    }

    pub fn into_parts(self) -> (Button, IconHandle) {
        (self.button, self.icon_handle)
    }
}

pub fn action_button(label: &str, icon_name: &str, tooltip: &str) -> ActionButton {
    let button = vp_button();
    button.add_css_class(button::CARD);
    button.add_css_class(control::BUTTON);
    button.set_hexpand(true);
    button.set_tooltip_text(Some(tooltip));

    let content = GtkBox::new(Orientation::Horizontal, 6);
    content.set_halign(Align::Center);
    content.set_valign(Align::Center);

    let icon = IconsService::global().create_icon(icon_name, &[control::BUTTON_ICON]);
    content.append(&icon.widget());

    let label = Label::new(Some(label));
    label.add_css_class(control::BUTTON_LABEL);
    content.append(&label);

    button.set_child(Some(&content));
    ActionButton {
        button,
        icon_handle: icon,
    }
}

pub fn set_active(button: &Button, active: bool) {
    button.remove_css_class(button::ACCENT);
    button.remove_css_class(button::CARD);
    button.remove_css_class(state::SERVICE_UNAVAILABLE);
    button.add_css_class(if active { button::ACCENT } else { button::CARD });
}

pub fn set_error(button: &Button, tooltip: &str) {
    button.add_css_class(state::SERVICE_UNAVAILABLE);
    button.set_tooltip_text(Some(tooltip));
}

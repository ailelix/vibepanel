//! Compact hardware and service control panel CSS.

pub fn css() -> &'static str {
    r#"
/* ===== COMMAND-BACKED CONTROL PANELS ===== */

.control-panel {
    min-width: 300px;
}

.control-panel-button {
    min-width: 0;
    min-height: 36px;
    border-radius: var(--radius-widget);
    font-size: var(--font-size-sm);
}

.control-panel-button > overlay > box {
    padding: 7px 10px;
}

button.control-panel-button label {
    margin: 0;
}

.control-panel-button-icon {
    font-size: 1em;
}

.control-panel-button.service-unavailable {
    color: var(--color-state-urgent);
}

.control-panel-button:hover {
    background: var(--color-card-overlay-hover);
}
"#
}

//! Command-backed control helpers for hardware profiles and systemd services.
//!
//! The GTK widgets run these synchronous methods through `gio::spawn_blocking`.
//! Keeping process execution here makes the UI code declarative and keeps all
//! command arguments explicit (none of these paths invoke a shell).

use std::ffi::OsStr;
use std::os::unix::process::CommandExt;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const NIRI_LOGOUT_ARGS: [&str; 3] = ["msg", "action", "quit"];

/// ASUS fan profiles supported by `asusctl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanProfile {
    Quiet,
    Balanced,
    Performance,
}

impl FanProfile {
    pub const ALL: [Self; 3] = [Self::Quiet, Self::Balanced, Self::Performance];

    pub fn label(self) -> &'static str {
        match self {
            Self::Quiet => "Quiet",
            Self::Balanced => "Balanced",
            Self::Performance => "Performance",
        }
    }

    fn command_value(self) -> &'static str {
        self.label()
    }
}

/// Graphics modes exposed by the ASUS control panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsMode {
    Integrated,
    Hybrid,
}

impl GraphicsMode {
    pub const ALL: [Self; 2] = [Self::Integrated, Self::Hybrid];

    pub fn label(self) -> &'static str {
        match self {
            Self::Integrated => "Integrated",
            Self::Hybrid => "Hybrid",
        }
    }

    fn command_value(self) -> &'static str {
        self.label()
    }
}

/// Process-backed ASUS controls.
#[derive(Debug, Clone)]
pub struct AsusControl {
    asusctl_command: String,
    supergfxctl_command: String,
    niri_command: String,
}

impl AsusControl {
    pub fn new(asusctl_command: String, supergfxctl_command: String, niri_command: String) -> Self {
        Self {
            asusctl_command,
            supergfxctl_command,
            niri_command,
        }
    }

    pub fn fan_profile(&self) -> Result<FanProfile, String> {
        let output = run_checked(&self.asusctl_command, ["profile", "get"])?;
        parse_fan_profile(&output)
            .ok_or_else(|| format!("Unrecognized asusctl profile output: {output}"))
    }

    pub fn set_fan_profile(&self, profile: FanProfile) -> Result<(), String> {
        run_checked(
            &self.asusctl_command,
            ["profile", "set", profile.command_value()],
        )?;
        Ok(())
    }

    pub fn graphics_mode(&self) -> Result<GraphicsMode, String> {
        let output = run_checked(&self.supergfxctl_command, ["-g"])?;
        parse_graphics_mode(&output)
            .ok_or_else(|| format!("Unrecognized supergfxctl mode output: {output}"))
    }

    pub fn pending_graphics_mode(&self) -> Result<Option<GraphicsMode>, String> {
        let output = run_checked(&self.supergfxctl_command, ["-P"])?;
        parse_pending_graphics_mode(&output)
    }

    pub fn set_graphics_mode(&self, mode: GraphicsMode) -> Result<(), String> {
        run_checked(&self.supergfxctl_command, ["-m", mode.command_value()])?;
        Ok(())
    }

    /// Ask Niri to show its native logout confirmation.
    pub fn request_logout_confirmation(&self) -> Result<(), String> {
        run_checked(&self.niri_command, NIRI_LOGOUT_ARGS)?;
        Ok(())
    }
}

/// Process-backed systemd service controls.
#[derive(Debug, Clone)]
pub struct SystemdControl {
    sudo_command: String,
    systemctl_command: String,
}

impl SystemdControl {
    pub fn new(sudo_command: String, systemctl_command: String) -> Self {
        Self {
            sudo_command,
            systemctl_command,
        }
    }

    /// Querying service state does not require elevated privileges.
    pub fn is_active(&self, unit: &str) -> Result<bool, String> {
        let output = run_output(&self.systemctl_command, ["is-active", unit])?;
        let state = String::from_utf8_lossy(&output.stdout);
        let state = state.trim();

        if let Some(active) = classify_systemd_state(state, output.status.code()) {
            return Ok(active);
        }

        if output.status.code() == Some(4) {
            return Err(format!("systemd unit not found: {unit}"));
        }

        Err(output_error(&self.systemctl_command, &output))
    }

    /// Start or stop a system service through non-interactive sudo.
    ///
    /// `-n` is intentional: vibepanel must never hang on a hidden terminal
    /// password prompt. Passwordless sudo succeeds directly; other setups get
    /// an actionable error in the panel tooltip.
    pub fn set_active(&self, unit: &str, active: bool) -> Result<bool, String> {
        let action = if active { "start" } else { "stop" };
        run_checked(
            &self.sudo_command,
            ["-n", self.systemctl_command.as_str(), action, unit],
        )?;
        self.is_active(unit)
    }
}

fn parse_fan_profile(output: &str) -> Option<FanProfile> {
    let normalized = output.to_ascii_lowercase();
    if normalized.contains("performance") {
        Some(FanProfile::Performance)
    } else if normalized.contains("balanced") {
        Some(FanProfile::Balanced)
    } else if normalized.contains("quiet") {
        Some(FanProfile::Quiet)
    } else {
        None
    }
}

fn parse_graphics_mode(output: &str) -> Option<GraphicsMode> {
    let normalized = output.to_ascii_lowercase();
    if normalized.contains("integrated") {
        Some(GraphicsMode::Integrated)
    } else if normalized.contains("hybrid") {
        Some(GraphicsMode::Hybrid)
    } else {
        None
    }
}

fn parse_pending_graphics_mode(output: &str) -> Result<Option<GraphicsMode>, String> {
    let normalized = output.trim().to_ascii_lowercase();
    if matches!(
        normalized.as_str(),
        "unknown" | "none" | "no action required"
    ) {
        return Ok(None);
    }

    parse_graphics_mode(&normalized)
        .map(Some)
        .ok_or_else(|| format!("Unsupported supergfxctl pending mode: {output}"))
}

fn classify_systemd_state(state: &str, exit_code: Option<i32>) -> Option<bool> {
    match exit_code {
        Some(0) => Some(state == "active"),
        // LSB status code 3 means the unit is known but not active. Code 4
        // means unknown, so it must remain an error even if stdout says inactive.
        Some(3) if matches!(state, "inactive" | "failed" | "deactivating" | "activating") => {
            Some(false)
        }
        _ => None,
    }
}

fn run_checked<I, S>(program: &str, args: I) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_output(program, args)?;
    if !output.status.success() {
        return Err(output_error(program, &output));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_output<I, S>(program: &str, args: I) -> Result<Output, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_output_with_timeout(program, args, COMMAND_TIMEOUT)
}

fn run_output_with_timeout<I, S>(
    program: &str,
    args: I,
    timeout: Duration,
) -> Result<Output, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let child = Command::new(program)
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| format!("Failed to run {program}: {error}"))?;

    let pid = child.id() as libc::pid_t;
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_for_watchdog = Arc::clone(&timed_out);
    let (cancel_tx, cancel_rx) = mpsc::channel::<()>();
    let watchdog = std::thread::spawn(move || {
        if cancel_rx.recv_timeout(timeout).is_err() {
            timed_out_for_watchdog.store(true, Ordering::SeqCst);
            // SAFETY: The child created a new process group whose ID is its PID.
            // Killing the group also terminates sudo/systemctl descendants.
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        }
    });

    let output = child.wait_with_output();
    let _ = cancel_tx.send(());
    let _ = watchdog.join();
    let output = output.map_err(|error| format!("Failed to wait for {program}: {error}"))?;

    if timed_out.load(Ordering::SeqCst) {
        return Err(format!(
            "{program} timed out after {} seconds",
            timeout.as_secs_f64()
        ));
    }

    Ok(output)
}

fn output_error(program: &str, output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stderr = stderr.trim();
    if !stderr.is_empty() {
        format!("{program}: {stderr}")
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stdout = stdout.trim();
        if stdout.is_empty() {
            format!("{program} exited with {}", output.status)
        } else {
            format!("{program}: {stdout}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_and_legacy_asusctl_profile_output() {
        assert_eq!(parse_fan_profile("Balanced\n"), Some(FanProfile::Balanced));
        assert_eq!(
            parse_fan_profile("Active profile is Performance"),
            Some(FanProfile::Performance)
        );
        assert_eq!(parse_fan_profile("Profile: quiet"), Some(FanProfile::Quiet));
        assert_eq!(parse_fan_profile("unknown"), None);
    }

    #[test]
    fn parses_supergfxctl_modes_case_insensitively() {
        assert_eq!(
            parse_graphics_mode("Integrated"),
            Some(GraphicsMode::Integrated)
        );
        assert_eq!(
            parse_graphics_mode("mode: HYBRID"),
            Some(GraphicsMode::Hybrid)
        );
        assert_eq!(parse_graphics_mode("Vfio"), None);
    }

    #[test]
    fn parses_pending_supergfxctl_modes_without_hiding_unsupported_modes() {
        assert_eq!(parse_pending_graphics_mode("Unknown"), Ok(None));
        assert_eq!(parse_pending_graphics_mode("None"), Ok(None));
        assert_eq!(
            parse_pending_graphics_mode("Hybrid"),
            Ok(Some(GraphicsMode::Hybrid))
        );
        assert!(parse_pending_graphics_mode("AsusMuxDgpu").is_err());
    }

    #[test]
    fn niri_logout_command_preserves_confirmation() {
        assert_eq!(NIRI_LOGOUT_ARGS, ["msg", "action", "quit"]);
        assert!(!NIRI_LOGOUT_ARGS.contains(&"-s"));
        assert!(!NIRI_LOGOUT_ARGS.contains(&"--skip-confirmation"));
    }

    #[test]
    fn classifies_normal_systemd_states_without_hiding_transport_errors() {
        assert_eq!(classify_systemd_state("active", Some(0)), Some(true));
        assert_eq!(classify_systemd_state("activating", Some(0)), Some(false));
        assert_eq!(classify_systemd_state("inactive", Some(3)), Some(false));
        assert_eq!(classify_systemd_state("failed", Some(3)), Some(false));
        assert_eq!(classify_systemd_state("inactive", Some(4)), None);
        assert_eq!(classify_systemd_state("", Some(1)), None);
        assert_eq!(classify_systemd_state("inactive", None), None);
    }

    #[test]
    fn command_runner_returns_output_before_timeout() {
        let output =
            run_output_with_timeout("sh", ["-c", "printf controlled"], Duration::from_secs(1))
                .expect("short command should complete");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), "controlled");
    }

    #[test]
    fn command_runner_kills_process_group_after_timeout() {
        let error =
            run_output_with_timeout("sh", ["-c", "sleep 5 & wait"], Duration::from_millis(50))
                .expect_err("long command should time out");
        assert!(error.contains("timed out"), "unexpected error: {error}");
    }
}

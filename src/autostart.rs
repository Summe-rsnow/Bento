//! Cross-platform launch-at-login ("开机自启") registration for desktop apps.
//!
//! - **Windows**: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` via direct Win32
//!   `advapi32` UTF-16 registry APIs (zero extra crate dependencies, no console flicker).
//! - **macOS**: `~/Library/LaunchAgents/<label>.plist`.
//! - **Linux**: `$XDG_CONFIG_HOME/autostart/<id>.desktop` (falling back to
//!   `~/.config/autostart/<id>.desktop`).

use std::io;
use std::path::{Path, PathBuf};

/// Configuration for registering or querying an application's launch-at-login entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoStartConfig {
    /// Application identifier (used as the Windows Run value name, Linux `.desktop`
    /// display name / filename stem, and macOS LaunchAgent label suffix).
    pub app_name: String,
    /// Optional explicit executable path. When `None`, [`std::env::current_exe`] is used.
    pub exe_path: Option<PathBuf>,
    /// Optional command-line arguments passed when the OS launches the app at login
    /// (for example `["--minimized"]` or `["--tray"]`).
    pub args: Vec<String>,
}

impl AutoStartConfig {
    /// Creates an auto-start configuration for `app_name` using the current executable
    /// and no extra arguments.
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            exe_path: None,
            args: Vec::new(),
        }
    }

    /// Overrides the executable path instead of resolving [`std::env::current_exe`].
    pub fn with_exe_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.exe_path = Some(path.into());
        self
    }

    /// Appends a single launch argument (e.g. `"--minimized"`).
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Appends multiple launch arguments.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Returns whether launch-at-login is currently registered for this app.
    pub fn is_enabled(&self) -> io::Result<bool> {
        let name = validate_app_name(&self.app_name)?;
        platform_is_enabled(name)
    }

    /// Registers launch-at-login for this app, overwriting any previous entry with the same name.
    pub fn enable(&self) -> io::Result<()> {
        let name = validate_app_name(&self.app_name)?;
        let exe = self.resolve_exe_path()?;
        platform_enable(name, &exe, &self.args)
    }

    /// Removes the launch-at-login registration for this app. Succeeds if already disabled.
    pub fn disable(&self) -> io::Result<()> {
        let name = validate_app_name(&self.app_name)?;
        platform_disable(name)
    }

    /// Enables or disables launch-at-login according to `enabled`.
    pub fn set_enabled(&self, enabled: bool) -> io::Result<()> {
        if enabled {
            self.enable()
        } else {
            self.disable()
        }
    }

    fn resolve_exe_path(&self) -> io::Result<PathBuf> {
        let path = match &self.exe_path {
            Some(p) => p.clone(),
            None => std::env::current_exe()?,
        };
        if path.as_os_str().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "autostart executable path cannot be empty",
            ));
        }
        if path.to_string_lossy().contains('\0') {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "autostart executable path cannot contain NUL characters",
            ));
        }
        Ok(path)
    }
}

/// Returns whether launch-at-login is currently enabled for `app_name`.
pub fn is_autostart_enabled(app_name: &str) -> io::Result<bool> {
    AutoStartConfig::new(app_name).is_enabled()
}

/// Registers the current executable to launch at user login under `app_name`.
pub fn enable_autostart(app_name: &str) -> io::Result<()> {
    AutoStartConfig::new(app_name).enable()
}

/// Removes the launch-at-login registration for `app_name`. Succeeds if already disabled.
pub fn disable_autostart(app_name: &str) -> io::Result<()> {
    AutoStartConfig::new(app_name).disable()
}

/// Enables or disables launch-at-login for `app_name` using the current executable.
pub fn set_autostart_enabled(app_name: &str, enabled: bool) -> io::Result<()> {
    AutoStartConfig::new(app_name).set_enabled(enabled)
}

fn validate_app_name(raw: &str) -> io::Result<&str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "autostart app_name cannot be empty",
        ));
    }
    if trimmed.contains(['\0', '/', '\\', '\r', '\n']) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "autostart app_name contains invalid characters",
        ));
    }
    Ok(trimmed)
}

/// Strips Windows extended-length path prefixes (`\\?\` / `\\?\UNC\`) that Explorer's
/// `Run` key launcher does not accept cleanly.
#[cfg(any(target_os = "windows", test))]
fn strip_windows_verbatim_prefix(raw: &str) -> String {
    if let Some(unc) = raw.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        rest.to_owned()
    } else {
        raw.to_owned()
    }
}

/// Quotes a single Windows command-line token following `CommandLineToArgvW` rules.
#[cfg(any(target_os = "windows", test))]
fn quote_windows_arg(arg: &str, always_quote: bool) -> String {
    let needs_quotes = always_quote
        || arg.is_empty()
        || arg.chars().any(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '"'));
    if !needs_quotes {
        return arg.to_owned();
    }

    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => {
                backslashes += 1;
            }
            '"' => {
                for _ in 0..(backslashes * 2 + 1) {
                    out.push('\\');
                }
                backslashes = 0;
                out.push('"');
            }
            _ => {
                for _ in 0..backslashes {
                    out.push('\\');
                }
                backslashes = 0;
                out.push(ch);
            }
        }
    }
    for _ in 0..(backslashes * 2) {
        out.push('\\');
    }
    out.push('"');
    out
}

/// Formats a Windows `HKCU\...\Run` command string with the executable always quoted
/// and any arguments safely escaped.
#[cfg(any(target_os = "windows", test))]
fn format_windows_run_command(exe: &Path, args: &[String]) -> String {
    let exe_str = strip_windows_verbatim_prefix(&exe.to_string_lossy());
    let mut out = quote_windows_arg(&exe_str, true);
    for arg in args {
        out.push(' ');
        out.push_str(&quote_windows_arg(arg, false));
    }
    out
}

/// Sanitizes an application name into a safe filename/identifier stem (`[A-Za-z0-9._-]`).
#[cfg(any(not(target_os = "windows"), test))]
fn sanitize_identifier(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut prev_dash = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches(['-', '.']);
    if trimmed.is_empty() {
        "app".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Derives the macOS LaunchAgent label for `app_name`.
///
/// Reverse-DNS names containing `.` are preserved (sanitized); plain names become
/// `com.bento.<lowercase_sanitized>`.
#[cfg(any(target_os = "macos", test))]
fn macos_launch_agent_label(app_name: &str) -> String {
    let sanitized = sanitize_identifier(app_name);
    if sanitized.contains('.') {
        sanitized
    } else {
        format!("com.bento.{}", sanitized.to_ascii_lowercase())
    }
}

#[cfg(any(target_os = "macos", test))]
fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Renders a macOS `LaunchAgents` `.plist` file for `label`, `exe`, and `args`.
#[cfg(any(target_os = "macos", test))]
fn render_macos_plist(label: &str, exe: &Path, args: &[String]) -> String {
    let mut program_args = String::new();
    program_args.push_str(&format!(
        "        <string>{}</string>\n",
        xml_escape(&exe.to_string_lossy())
    ));
    for arg in args {
        program_args.push_str(&format!(
            "        <string>{}</string>\n",
            xml_escape(arg)
        ));
    }

    format!(
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n",
            "<plist version=\"1.0\">\n",
            "<dict>\n",
            "    <key>Label</key>\n",
            "    <string>{label}</string>\n",
            "    <key>ProgramArguments</key>\n",
            "    <array>\n",
            "{program_args}",
            "    </array>\n",
            "    <key>RunAtLoad</key>\n",
            "    <true/>\n",
            "</dict>\n",
            "</plist>\n",
        ),
        label = xml_escape(label),
        program_args = program_args,
    )
}

#[cfg(any(all(unix, not(target_os = "macos")), test))]
fn quote_desktop_exec_arg(arg: &str) -> String {
    // Per FreeDesktop Desktop Entry Spec: double-quote if empty or containing reserved chars,
    // escaping `"`, `` ` ``, `$`, `\` with a backslash and `%` as `%%`.
    let needs_quotes = arg.is_empty()
        || arg.chars().any(|c| {
            matches!(
                c,
                ' ' | '\t'
                    | '\n'
                    | '"'
                    | '\''
                    | '\\'
                    | '>'
                    | '<'
                    | '~'
                    | '|'
                    | '&'
                    | ';'
                    | '$'
                    | '*'
                    | '?'
                    | '#'
                    | '('
                    | ')'
                    | '`'
            )
        });

    let mut out = String::with_capacity(arg.len() + 2);
    if needs_quotes {
        out.push('"');
    }
    for ch in arg.chars() {
        match ch {
            '"' | '`' | '$' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '%' => out.push_str("%%"),
            _ => out.push(ch),
        }
    }
    if needs_quotes {
        out.push('"');
    }
    out
}

/// Renders an XDG autostart `.desktop` entry for `app_name`, `exe`, and `args`.
#[cfg(any(all(unix, not(target_os = "macos")), test))]
fn render_linux_desktop_entry(app_name: &str, exe: &Path, args: &[String]) -> String {
    let mut exec = quote_desktop_exec_arg(&exe.to_string_lossy());
    for arg in args {
        exec.push(' ');
        exec.push_str(&quote_desktop_exec_arg(arg));
    }
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name={app_name}\n\
         Exec={exec}\n\
         X-GNOME-Autostart-enabled=true\n"
    )
}

// ---------------------------------------------------------------------------
// Platform backends
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod win {
    use std::ffi::c_void;
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::ptr;

    use super::format_windows_run_command;

    type Hkey = *mut c_void;

    // `#define HKEY_CURRENT_USER ((HKEY)(ULONG_PTR)((LONG)0x80000001))`
    const HKEY_CURRENT_USER: Hkey = (0x8000_0001_u32 as i32 as isize) as Hkey;
    const ERROR_SUCCESS: i32 = 0;
    const ERROR_FILE_NOT_FOUND: i32 = 2;
    const KEY_QUERY_VALUE: u32 = 0x0001;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_OPTION_NON_VOLATILE: u32 = 0;
    const REG_SZ: u32 = 1;
    const REG_EXPAND_SZ: u32 = 2;

    pub(super) const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn RegOpenKeyExW(
            h_key: Hkey,
            lp_sub_key: *const u16,
            ul_options: u32,
            sam_desired: u32,
            phk_result: *mut Hkey,
        ) -> i32;
        fn RegCreateKeyExW(
            h_key: Hkey,
            lp_sub_key: *const u16,
            reserved: u32,
            lp_class: *const u16,
            dw_options: u32,
            sam_desired: u32,
            lp_security_attributes: *const c_void,
            phk_result: *mut Hkey,
            lpdw_disposition: *mut u32,
        ) -> i32;
        fn RegQueryValueExW(
            h_key: Hkey,
            lp_value_name: *const u16,
            lp_reserved: *const u32,
            lp_type: *mut u32,
            lp_data: *mut u8,
            lpcb_data: *mut u32,
        ) -> i32;
        fn RegSetValueExW(
            h_key: Hkey,
            lp_value_name: *const u16,
            reserved: u32,
            dw_type: u32,
            lp_data: *const u8,
            cb_data: u32,
        ) -> i32;
        fn RegDeleteValueW(h_key: Hkey, lp_value_name: *const u16) -> i32;
        #[cfg(test)]
        fn RegDeleteKeyW(h_key: Hkey, lp_sub_key: *const u16) -> i32;
        fn RegCloseKey(h_key: Hkey) -> i32;
    }

    struct RegKey(Hkey);

    impl Drop for RegKey {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: `self.0` is an open registry key handle returned by `RegOpenKeyExW` or `RegCreateKeyExW`.
                unsafe {
                    RegCloseKey(self.0);
                }
            }
        }
    }

    fn to_wide(s: &str) -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    pub(super) fn is_enabled_in(subkey: &str, value_name: &str) -> io::Result<bool> {
        let subkey_w = to_wide(subkey);
        let mut raw_key: Hkey = ptr::null_mut();
        // SAFETY: `subkey_w` is NUL-terminated UTF-16; `raw_key` receives the opened handle.
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey_w.as_ptr(),
                0,
                KEY_QUERY_VALUE,
                &mut raw_key,
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status));
        }
        let key = RegKey(raw_key);

        let name_w = to_wide(value_name);
        let mut data_type: u32 = 0;
        let mut data_len: u32 = 0;
        // SAFETY: `key.0` is valid, `name_w` is NUL-terminated UTF-16, and passing null `lp_data`
        // queries the value's existence, type, and byte length without writing a buffer.
        let q_status = unsafe {
            RegQueryValueExW(
                key.0,
                name_w.as_ptr(),
                ptr::null(),
                &mut data_type,
                ptr::null_mut(),
                &mut data_len,
            )
        };
        if q_status == ERROR_FILE_NOT_FOUND {
            return Ok(false);
        }
        if q_status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(q_status));
        }
        Ok((data_type == REG_SZ || data_type == REG_EXPAND_SZ) && data_len > 2)
    }

    pub(super) fn enable_in(
        subkey: &str,
        value_name: &str,
        exe: &Path,
        args: &[String],
    ) -> io::Result<()> {
        let subkey_w = to_wide(subkey);
        let mut raw_key: Hkey = ptr::null_mut();
        // SAFETY: `subkey_w` is NUL-terminated UTF-16; optional pointers are null.
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                subkey_w.as_ptr(),
                0,
                ptr::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                ptr::null(),
                &mut raw_key,
                ptr::null_mut(),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status));
        }
        let key = RegKey(raw_key);

        let command = format_windows_run_command(exe, args);
        let name_w = to_wide(value_name);
        let data_w = to_wide(&command);
        let cb_data = u32::try_from(data_w.len() * std::mem::size_of::<u16>()).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "autostart command too long")
        })?;
        // SAFETY: `key.0` is valid; `name_w` and `data_w` are NUL-terminated UTF-16 buffers
        // whose exact byte length is passed in `cb_data`.
        let set_status = unsafe {
            RegSetValueExW(
                key.0,
                name_w.as_ptr(),
                0,
                REG_SZ,
                data_w.as_ptr().cast::<u8>(),
                cb_data,
            )
        };
        if set_status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(set_status));
        }
        Ok(())
    }

    pub(super) fn disable_in(subkey: &str, value_name: &str) -> io::Result<()> {
        let subkey_w = to_wide(subkey);
        let mut raw_key: Hkey = ptr::null_mut();
        // SAFETY: `subkey_w` is NUL-terminated UTF-16; `raw_key` receives the opened handle.
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                subkey_w.as_ptr(),
                0,
                KEY_SET_VALUE,
                &mut raw_key,
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(());
        }
        if status != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(status));
        }
        let key = RegKey(raw_key);

        let name_w = to_wide(value_name);
        // SAFETY: `key.0` is valid and `name_w` is NUL-terminated UTF-16.
        let del_status = unsafe { RegDeleteValueW(key.0, name_w.as_ptr()) };
        if del_status == ERROR_SUCCESS || del_status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(io::Error::from_raw_os_error(del_status))
        }
    }

    #[cfg(test)]
    pub(super) fn delete_test_subkey(subkey: &str) {
        let subkey_w = to_wide(subkey);
        // SAFETY: `subkey_w` is NUL-terminated UTF-16; test-only cleanup of a temporary subkey.
        unsafe {
            let _ = RegDeleteKeyW(HKEY_CURRENT_USER, subkey_w.as_ptr());
        }
    }
}

#[cfg(target_os = "windows")]
fn platform_is_enabled(app_name: &str) -> io::Result<bool> {
    win::is_enabled_in(win::RUN_SUBKEY, app_name)
}

#[cfg(target_os = "windows")]
fn platform_enable(app_name: &str, exe: &Path, args: &[String]) -> io::Result<()> {
    win::enable_in(win::RUN_SUBKEY, app_name, exe, args)
}

#[cfg(target_os = "windows")]
fn platform_disable(app_name: &str) -> io::Result<()> {
    win::disable_in(win::RUN_SUBKEY, app_name)
}

#[cfg(target_os = "macos")]
fn macos_plist_path(app_name: &str) -> io::Result<(String, PathBuf)> {
    let home = std::env::var_os("HOME").ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "HOME environment variable is not set")
    })?;
    let label = macos_launch_agent_label(app_name);
    let path = PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{label}.plist"));
    Ok((label, path))
}

#[cfg(target_os = "macos")]
fn platform_is_enabled(app_name: &str) -> io::Result<bool> {
    let (_, path) = macos_plist_path(app_name)?;
    Ok(path.is_file())
}

#[cfg(target_os = "macos")]
fn platform_enable(app_name: &str, exe: &Path, args: &[String]) -> io::Result<()> {
    let (label, path) = macos_plist_path(app_name)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = render_macos_plist(&label, exe, args);
    std::fs::write(path, content)
}

#[cfg(target_os = "macos")]
fn platform_disable(app_name: &str) -> io::Result<()> {
    let (_, path) = macos_plist_path(app_name)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn linux_desktop_entry_path(app_name: &str) -> io::Result<PathBuf> {
    let config_dir = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => {
            let home = std::env::var_os("HOME").ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "neither XDG_CONFIG_HOME nor HOME environment variable is set",
                )
            })?;
            PathBuf::from(home).join(".config")
        }
    };
    let stem = sanitize_identifier(app_name).to_ascii_lowercase();
    Ok(config_dir.join("autostart").join(format!("{stem}.desktop")))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_is_enabled(app_name: &str) -> io::Result<bool> {
    let path = linux_desktop_entry_path(app_name)?;
    Ok(path.is_file())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_enable(app_name: &str, exe: &Path, args: &[String]) -> io::Result<()> {
    let path = linux_desktop_entry_path(app_name)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = render_linux_desktop_entry(app_name, exe, args);
    std::fs::write(path, content)
}

#[cfg(all(unix, not(target_os = "macos")))]
fn platform_disable(app_name: &str) -> io::Result<()> {
    let path = linux_desktop_entry_path(app_name)?;
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

#[cfg(not(any(target_os = "windows", unix)))]
fn platform_is_enabled(_app_name: &str) -> io::Result<bool> {
    Ok(false)
}

#[cfg(not(any(target_os = "windows", unix)))]
fn platform_enable(_app_name: &str, _exe: &Path, _args: &[String]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "autostart is not supported on this platform",
    ))
}

#[cfg(not(any(target_os = "windows", unix)))]
fn platform_disable(_app_name: &str) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_app_names_and_rejects_invalid_inputs() {
        assert_eq!(validate_app_name("Wonton").unwrap(), "Wonton");
        assert_eq!(validate_app_name("  Ramen  ").unwrap(), "Ramen");
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("   ").is_err());
        assert!(validate_app_name("bad/name").is_err());
        assert!(validate_app_name(r"bad\name").is_err());
        assert!(validate_app_name("bad\0name").is_err());
        assert!(validate_app_name("bad\nname").is_err());
    }

    #[test]
    fn formats_windows_run_command_with_quoting_and_verbatim_stripping() {
        assert_eq!(
            format_windows_run_command(Path::new(r"C:\Program Files\Wonton\wonton.exe"), &[]),
            r#""C:\Program Files\Wonton\wonton.exe""#
        );
        assert_eq!(
            format_windows_run_command(
                Path::new(r"\\?\C:\Users\Alice\AppData\Local\Ramen\ramen.exe"),
                &["--minimized".to_owned(), "--profile=my work".to_owned()],
            ),
            r#""C:\Users\Alice\AppData\Local\Ramen\ramen.exe" --minimized "--profile=my work""#
        );
        assert_eq!(
            format_windows_run_command(
                Path::new(r"\\?\UNC\server\share\Sundae\sundae.exe"),
                &[r#"say "hi""#.to_owned()],
            ),
            r#""\\server\share\Sundae\sundae.exe" "say \"hi\"""#
        );
    }

    #[test]
    fn renders_macos_launch_agent_plist_with_xml_escaping() {
        assert_eq!(macos_launch_agent_label("Wonton"), "com.bento.wonton");
        assert_eq!(
            macos_launch_agent_label("com.example.Ramen"),
            "com.example.Ramen"
        );
        assert_eq!(
            macos_launch_agent_label("My Cool App!"),
            "com.bento.my-cool-app"
        );

        let plist = render_macos_plist(
            "com.bento.wonton",
            Path::new("/Applications/Wonton & Co.app/Contents/MacOS/wonton"),
            &["--tray".to_owned(), "--tag=<dev>".to_owned()],
        );
        assert!(plist.contains("<key>Label</key>\n    <string>com.bento.wonton</string>"));
        assert!(plist.contains("<string>/Applications/Wonton &amp; Co.app/Contents/MacOS/wonton</string>"));
        assert!(plist.contains("<string>--tray</string>"));
        assert!(plist.contains("<string>--tag=&lt;dev&gt;</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>\n    <true/>"));
    }

    #[test]
    fn renders_linux_desktop_entry_with_exec_escaping() {
        let entry = render_linux_desktop_entry(
            "Sundae",
            Path::new("/opt/My Apps/sundae"),
            &["--minimized".to_owned(), "--fmt=%s$1".to_owned()],
        );
        assert!(entry.contains("[Desktop Entry]\n"));
        assert!(entry.contains("Type=Application\n"));
        assert!(entry.contains("Name=Sundae\n"));
        assert!(entry.contains("Exec=\"/opt/My Apps/sundae\" --minimized \"--fmt=%%s\\$1\"\n"));
        assert!(entry.contains("X-GNOME-Autostart-enabled=true\n"));
    }

    #[test]
    fn non_existent_app_queries_false_and_disables_idempotently() {
        let unique = format!("BentoNonExistentProbe_{}", std::process::id());
        assert!(!is_autostart_enabled(&unique).expect("query non-existent entry"));
        disable_autostart(&unique).expect("disable non-existent entry is idempotent");
    }

    #[test]
    fn config_builder_preserves_fields_and_rejects_empty_exe_path() {
        let cfg = AutoStartConfig::new("Wonton")
            .with_exe_path(r"C:\Apps\wonton.exe")
            .with_arg("--minimized")
            .with_args(["--silent"]);
        assert_eq!(cfg.app_name, "Wonton");
        assert_eq!(cfg.exe_path.as_deref(), Some(Path::new(r"C:\Apps\wonton.exe")));
        assert_eq!(cfg.args, vec!["--minimized", "--silent"]);

        let empty_exe = AutoStartConfig::new("Wonton").with_exe_path("");
        assert!(empty_exe.enable().is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_registry_round_trips_safely_in_isolated_test_subkey() {
        struct TestKeyGuard(String);
        impl Drop for TestKeyGuard {
            fn drop(&mut self) {
                win::delete_test_subkey(&self.0);
            }
        }

        let subkey = format!(r"Software\BentoAutostartUnitTest_{}", std::process::id());
        let _guard = TestKeyGuard(subkey.clone());

        assert!(!win::is_enabled_in(&subkey, "WontonTest").unwrap());
        win::enable_in(
            &subkey,
            "WontonTest",
            Path::new(r"C:\Program Files\Wonton\wonton.exe"),
            &["--minimized".to_owned()],
        )
        .unwrap();
        assert!(win::is_enabled_in(&subkey, "WontonTest").unwrap());

        win::disable_in(&subkey, "WontonTest").unwrap();
        assert!(!win::is_enabled_in(&subkey, "WontonTest").unwrap());
        // Second disable is idempotent.
        win::disable_in(&subkey, "WontonTest").unwrap();
    }
}

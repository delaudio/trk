use std::{
    env,
    process::{Child, Command},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrowserPlatform {
    MacOs,
    Windows,
    Unix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BrowserCommand {
    executable: &'static str,
    arguments: Vec<String>,
}

pub(crate) struct BrowserOpenMonitor {
    child: Child,
    url: String,
}

impl BrowserOpenMonitor {
    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn try_result(&mut self) -> Option<Result<(), String>> {
        match self.child.try_wait() {
            Ok(Some(status)) if status.success() => Some(Ok(())),
            Ok(Some(status)) => Some(Err(format!("browser opener exited with {status}"))),
            Ok(None) => None,
            Err(error) => Some(Err(format!("failed to wait for browser opener: {error}"))),
        }
    }
}

impl Drop for BrowserOpenMonitor {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

pub(crate) fn open_browser(url: &str) -> Result<BrowserOpenMonitor, String> {
    let platform = current_platform();
    if !graphical_session_available(platform, |name| env::var_os(name).is_some()) {
        return Err("no graphical session detected".to_string());
    }
    let command = browser_command(platform, url);
    let child = Command::new(command.executable)
        .args(&command.arguments)
        .spawn()
        .map_err(|error| format!("failed to start browser opener: {error}"))?;
    Ok(BrowserOpenMonitor {
        child,
        url: url.to_string(),
    })
}

fn current_platform() -> BrowserPlatform {
    if cfg!(target_os = "macos") {
        BrowserPlatform::MacOs
    } else if cfg!(target_os = "windows") {
        BrowserPlatform::Windows
    } else {
        BrowserPlatform::Unix
    }
}

fn browser_command(platform: BrowserPlatform, url: &str) -> BrowserCommand {
    match platform {
        BrowserPlatform::MacOs => BrowserCommand {
            executable: "open",
            arguments: vec![url.to_string()],
        },
        BrowserPlatform::Windows => BrowserCommand {
            executable: "cmd.exe",
            arguments: vec![
                "/D".to_string(),
                "/S".to_string(),
                "/C".to_string(),
                "start".to_string(),
                String::new(),
                url.to_string(),
            ],
        },
        BrowserPlatform::Unix => BrowserCommand {
            executable: "xdg-open",
            arguments: vec![url.to_string()],
        },
    }
}

fn graphical_session_available(
    platform: BrowserPlatform,
    mut present: impl FnMut(&str) -> bool,
) -> bool {
    match platform {
        BrowserPlatform::MacOs | BrowserPlatform::Windows => true,
        BrowserPlatform::Unix => {
            present("DISPLAY") || present("WAYLAND_DISPLAY") || present("MIR_SOCKET")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opener_plans_keep_the_url_as_one_argument_on_every_platform() {
        let url = "http://127.0.0.1:3334/";
        let mac = browser_command(BrowserPlatform::MacOs, url);
        assert_eq!(mac.executable, "open");
        assert_eq!(mac.arguments, [url]);

        let unix = browser_command(BrowserPlatform::Unix, url);
        assert_eq!(unix.executable, "xdg-open");
        assert_eq!(unix.arguments, [url]);

        let windows = browser_command(BrowserPlatform::Windows, url);
        assert_eq!(windows.executable, "cmd.exe");
        assert_eq!(windows.arguments.last().map(String::as_str), Some(url));
        assert_eq!(windows.arguments[4], "");
    }

    #[test]
    fn unix_headless_detection_accepts_either_display_protocol() {
        assert!(!graphical_session_available(BrowserPlatform::Unix, |_| {
            false
        }));
        assert!(graphical_session_available(BrowserPlatform::Unix, |name| {
            name == "DISPLAY"
        }));
        assert!(graphical_session_available(BrowserPlatform::Unix, |name| {
            name == "WAYLAND_DISPLAY"
        }));
        assert!(graphical_session_available(BrowserPlatform::MacOs, |_| {
            false
        }));
    }
}

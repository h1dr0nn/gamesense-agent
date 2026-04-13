// ADB Command Builder - Typed builder for ADB commands
// Provides a fluent API to construct ADB commands with type safety.

/// Represents a variety of ADB commands
#[derive(Debug, Clone)]
pub enum AdbCommand {
    Version,
    Devices { long: bool },
    Shell(Vec<String>),
    Install { path: String, reinstall: bool },
    Uninstall { package: String, keep_data: bool },
    Push { local: String, remote: String },
    Pull { remote: String, local: String },
    Reboot { mode: Option<String> },
    StartServer,
    KillServer,
    GetProp(String),
}

impl AdbCommand {
    /// Convert the command into a vector of arguments for the ADB process
    pub fn to_args(&self) -> Vec<String> {
        match self {
            AdbCommand::Version => vec!["version".into()],
            AdbCommand::Devices { long } => {
                let mut args = vec!["devices".into()];
                if *long {
                    args.push("-l".into());
                }
                args
            }
            AdbCommand::Shell(shell_args) => {
                let mut args = vec!["shell".into()];
                args.extend(shell_args.iter().cloned());
                args
            }
            AdbCommand::Install { path, reinstall } => {
                let mut args = vec!["install".into()];
                if *reinstall {
                    args.push("-r".into());
                }
                args.push(path.clone());
                args
            }
            AdbCommand::Uninstall { package, keep_data } => {
                let mut args = vec!["uninstall".into()];
                if *keep_data {
                    args.push("-k".into());
                }
                args.push(package.clone());
                args
            }
            AdbCommand::Push { local, remote } => {
                vec!["push".into(), local.clone(), remote.clone()]
            }
            AdbCommand::Pull { remote, local } => {
                vec!["pull".into(), remote.clone(), local.clone()]
            }
            AdbCommand::Reboot { mode } => {
                let mut args = vec!["reboot".into()];
                if let Some(m) = mode {
                    args.push(m.clone());
                }
                args
            }
            AdbCommand::StartServer => vec!["start-server".into()],
            AdbCommand::KillServer => vec!["kill-server".into()],
            AdbCommand::GetProp(prop) => vec!["shell".into(), "getprop".into(), prop.clone()],
        }
    }
}

/// Builder for constructing ADB commands targeting specific devices
pub struct AdbCommandBuilder {
    device_id: Option<String>,
}

impl AdbCommandBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self { device_id: None }
    }

    /// Target a specific device by its ID
    pub fn target(mut self, device_id: &str) -> Self {
        self.device_id = Some(device_id.to_string());
        self
    }

    /// Construct a full argument list including device targeting strings
    pub fn build(&self, command: AdbCommand) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(ref id) = self.device_id {
            args.push("-s".into());
            args.push(id.clone());
        }

        args.extend(command.to_args());
        args
    }
}

/// Helper to quickly build common shell commands
pub struct ShellCommandBuilder {
    args: Vec<String>,
}

impl ShellCommandBuilder {
    pub fn new(command: &str) -> Self {
        Self {
            args: vec![command.to_string()],
        }
    }

    pub fn arg(mut self, value: &str) -> Self {
        self.args.push(value.to_string());
        self
    }

    pub fn build(self) -> Vec<String> {
        self.args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_to_args() {
        let cmd = AdbCommand::Devices { long: true };
        assert_eq!(cmd.to_args(), vec!["devices", "-l"]);
    }

    #[test]
    fn devices_short_has_no_flag() {
        let cmd = AdbCommand::Devices { long: false };
        assert_eq!(cmd.to_args(), vec!["devices"]);
    }

    #[test]
    fn version_command() {
        assert_eq!(AdbCommand::Version.to_args(), vec!["version"]);
    }

    #[test]
    fn install_without_reinstall() {
        let cmd = AdbCommand::Install {
            path: "/tmp/app.apk".into(),
            reinstall: false,
        };
        assert_eq!(cmd.to_args(), vec!["install", "/tmp/app.apk"]);
    }

    #[test]
    fn install_with_reinstall() {
        let cmd = AdbCommand::Install {
            path: "/tmp/app.apk".into(),
            reinstall: true,
        };
        assert_eq!(cmd.to_args(), vec!["install", "-r", "/tmp/app.apk"]);
    }

    #[test]
    fn uninstall_with_keep_data() {
        let cmd = AdbCommand::Uninstall {
            package: "com.example.app".into(),
            keep_data: true,
        };
        assert_eq!(cmd.to_args(), vec!["uninstall", "-k", "com.example.app"]);
    }

    #[test]
    fn uninstall_without_keep_data() {
        let cmd = AdbCommand::Uninstall {
            package: "com.example.app".into(),
            keep_data: false,
        };
        assert_eq!(cmd.to_args(), vec!["uninstall", "com.example.app"]);
    }

    #[test]
    fn push_command() {
        let cmd = AdbCommand::Push {
            local: "/tmp/file.txt".into(),
            remote: "/sdcard/file.txt".into(),
        };
        assert_eq!(cmd.to_args(), vec!["push", "/tmp/file.txt", "/sdcard/file.txt"]);
    }

    #[test]
    fn pull_command() {
        let cmd = AdbCommand::Pull {
            remote: "/sdcard/file.txt".into(),
            local: "/tmp/file.txt".into(),
        };
        assert_eq!(cmd.to_args(), vec!["pull", "/sdcard/file.txt", "/tmp/file.txt"]);
    }

    #[test]
    fn reboot_default() {
        let cmd = AdbCommand::Reboot { mode: None };
        assert_eq!(cmd.to_args(), vec!["reboot"]);
    }

    #[test]
    fn reboot_with_mode() {
        let cmd = AdbCommand::Reboot {
            mode: Some("bootloader".into()),
        };
        assert_eq!(cmd.to_args(), vec!["reboot", "bootloader"]);
    }

    #[test]
    fn start_server() {
        assert_eq!(AdbCommand::StartServer.to_args(), vec!["start-server"]);
    }

    #[test]
    fn kill_server() {
        assert_eq!(AdbCommand::KillServer.to_args(), vec!["kill-server"]);
    }

    #[test]
    fn getprop_command() {
        let cmd = AdbCommand::GetProp("ro.product.model".into());
        assert_eq!(cmd.to_args(), vec!["shell", "getprop", "ro.product.model"]);
    }

    #[test]
    fn shell_command() {
        let cmd = AdbCommand::Shell(vec!["ls".into(), "-la".into(), "/sdcard".into()]);
        assert_eq!(cmd.to_args(), vec!["shell", "ls", "-la", "/sdcard"]);
    }

    #[test]
    fn test_builder_with_device() {
        let builder = AdbCommandBuilder::new().target("12345");
        let args = builder.build(AdbCommand::Shell(vec!["ls".into(), "/sdcard".into()]));
        assert_eq!(args, vec!["-s", "12345", "shell", "ls", "/sdcard"]);
    }

    #[test]
    fn builder_without_device() {
        let builder = AdbCommandBuilder::new();
        let args = builder.build(AdbCommand::Version);
        assert_eq!(args, vec!["version"]);
    }

    #[test]
    fn test_shell_builder() {
        let args = ShellCommandBuilder::new("input")
            .arg("tap")
            .arg("100")
            .arg("200")
            .build();
        assert_eq!(args, vec!["input", "tap", "100", "200"]);
    }

    #[test]
    fn shell_builder_single_command() {
        let args = ShellCommandBuilder::new("dumpsys").build();
        assert_eq!(args, vec!["dumpsys"]);
    }
}

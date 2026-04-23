use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CliBusSelection {
    Auto,
    System,
    Session,
}

#[derive(Debug, Parser)]
#[command(
    name = "systemd-tui",
    version,
    about = "Interactive TUI for systemd over D-Bus"
)]
pub struct Cli {
    #[arg(
        long,
        env = "SYSTEMD_TUI_REFRESH_MS",
        default_value_t = 2000,
        help = "Polling refresh interval in milliseconds"
    )]
    pub refresh_ms: u64,

    #[arg(
        long,
        env = "SYSTEMD_TUI_BUS",
        value_enum,
        default_value_t = CliBusSelection::Auto,
        help = "Bus selection mode: auto/system/session"
    )]
    pub bus: CliBusSelection,

    #[arg(
        long,
        env = "SYSTEMD_TUI_SSH_HOST",
        help = "Remote SSH host for DBus tunneling"
    )]
    pub ssh_host: Option<String>,

    #[arg(long, env = "SYSTEMD_TUI_SSH_USER", help = "Remote SSH username")]
    pub ssh_user: Option<String>,

    #[arg(
        long,
        env = "SYSTEMD_TUI_SSH_PORT",
        default_value_t = 22,
        help = "Remote SSH port"
    )]
    pub ssh_port: u16,

    #[arg(long, env = "SYSTEMD_TUI_SSH_KEY", help = "Path to SSH private key")]
    pub ssh_key: Option<PathBuf>,

    #[arg(
        long,
        env = "SYSTEMD_TUI_DBUS_SOCKET",
        help = "DBus unix socket path override (applies to local and SSH connections when set)"
    )]
    pub dbus_socket: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{Cli, CliBusSelection};
    use clap::Parser;

    #[test]
    fn parses_defaults() {
        let cli = Cli::parse_from(["systemd-tui"]);
        assert_eq!(cli.refresh_ms, 2000);
        assert!(matches!(cli.bus, CliBusSelection::Auto));
        assert!(cli.ssh_host.is_none());
        assert!(cli.dbus_socket.is_none());
    }

    #[test]
    fn parses_ssh_options() {
        let cli = Cli::parse_from([
            "systemd-tui",
            "--bus",
            "system",
            "--ssh-host",
            "srv.example",
            "--ssh-user",
            "root",
            "--ssh-port",
            "2222",
            "--dbus-socket",
            "/var/run/dbus/system_bus_socket",
        ]);
        assert!(matches!(cli.bus, CliBusSelection::System));
        assert_eq!(cli.ssh_host.as_deref(), Some("srv.example"));
        assert_eq!(cli.ssh_user.as_deref(), Some("root"));
        assert_eq!(cli.ssh_port, 2222);
        assert_eq!(
            cli.dbus_socket.as_deref(),
            Some("/var/run/dbus/system_bus_socket")
        );
    }
}

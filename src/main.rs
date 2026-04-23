use anyhow::Result;
use clap::Parser;
use std::time::Duration;

mod actions;
mod app;
mod bus;
mod cli;
mod config;
mod tui;
mod ui;
mod units;

use app::App;
use bus::{BusSelection, SshConfig};
use cli::{Cli, CliBusSelection};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let connection = build_connection_config(&cli);
    let config = config::Config {
        refresh_interval: Duration::from_millis(cli.refresh_ms),
        connection,
        ..Default::default()
    };

    let mut app = App::new(config).await?;
    let mut terminal = tui::init()?;
    let run_result = app.run(&mut terminal).await;
    tui::restore()?;
    run_result
}

fn build_connection_config(cli: &Cli) -> crate::bus::ConnectionConfig {
    let bus = match cli.bus {
        CliBusSelection::Auto => BusSelection::Auto,
        CliBusSelection::System => BusSelection::System,
        CliBusSelection::Session => BusSelection::Session,
    };

    let ssh = cli.ssh_host.clone().map(|host| SshConfig {
        host,
        user: cli.ssh_user.clone(),
        port: cli.ssh_port,
        key_path: cli.ssh_key.clone(),
    });

    crate::bus::ConnectionConfig {
        bus,
        ssh,
        dbus_socket: cli.dbus_socket.clone(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::build_connection_config;
    use crate::bus::BusSelection;
    use crate::cli::Cli;
    use clap::Parser;

    #[test]
    fn build_connection_config_keeps_explicit_dbus_socket_for_auto_bus() {
        let cli = Cli::parse_from([
            "systemd-tui",
            "--bus",
            "auto",
            "--dbus-socket",
            "/run/user/1000/bus",
        ]);

        let cfg = build_connection_config(&cli);
        assert_eq!(cfg.bus, BusSelection::Auto);
        assert_eq!(cfg.dbus_socket.as_deref(), Some("/run/user/1000/bus"));
    }

    #[test]
    fn build_connection_config_keeps_explicit_dbus_socket_for_session_bus() {
        let cli = Cli::parse_from([
            "systemd-tui",
            "--bus",
            "session",
            "--dbus-socket",
            "/run/user/1000/bus",
        ]);

        let cfg = build_connection_config(&cli);
        assert_eq!(cfg.bus, BusSelection::Session);
        assert_eq!(cfg.dbus_socket.as_deref(), Some("/run/user/1000/bus"));
    }

    #[test]
    fn build_connection_config_preserves_ssh_config() {
        let cli = Cli::parse_from([
            "systemd-tui",
            "--ssh-host",
            "srv.example",
            "--ssh-user",
            "root",
            "--ssh-port",
            "2222",
            "--dbus-socket",
            "/var/run/dbus/system_bus_socket",
        ]);

        let cfg = build_connection_config(&cli);
        assert_eq!(
            cfg.ssh.as_ref().map(|s| s.host.as_str()),
            Some("srv.example")
        );
        assert_eq!(
            cfg.ssh.as_ref().and_then(|s| s.user.as_deref()),
            Some("root")
        );
        assert_eq!(cfg.ssh.as_ref().map(|s| s.port), Some(2222));
        assert_eq!(
            cfg.dbus_socket.as_deref(),
            Some("/var/run/dbus/system_bus_socket")
        );
    }
}

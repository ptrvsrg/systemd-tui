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
    let config = config::Config {
        refresh_interval: Duration::from_millis(cli.refresh_ms),
        connection: crate::bus::ConnectionConfig {
            bus: match cli.bus {
                CliBusSelection::Auto => BusSelection::Auto,
                CliBusSelection::System => BusSelection::System,
                CliBusSelection::Session => BusSelection::Session,
            },
            ssh: cli.ssh_host.map(|host| SshConfig {
                host,
                user: cli.ssh_user,
                port: cli.ssh_port,
                key_path: cli.ssh_key,
            }),
            dbus_socket: Some(cli.dbus_socket),
            ..Default::default()
        },
        ..Default::default()
    };

    let mut app = App::new(config).await?;
    let mut terminal = tui::init()?;
    let run_result = app.run(&mut terminal).await;
    tui::restore()?;
    run_result
}

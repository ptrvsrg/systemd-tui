use crate::bus::{BusKind, BusSelection, ConnectionConfig, SshConfig, SshTunnel};
use crate::units::structs::{SystemdUnit, UnitActiveState, UnitLoadState};
use anyhow::{Context, Result};
use futures::StreamExt;
use std::process::Stdio;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::process::Command;
use zbus::Connection;
use zbus::connection::Builder as ConnectionBuilder;
use zbus::zvariant::OwnedObjectPath;

type ListUnitsRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    OwnedObjectPath,
    u32,
    String,
    OwnedObjectPath,
);

pub struct SystemdManager {
    conn: Connection,
    bus_kind: BusKind,
    ssh_config: Option<SshConfig>,
    command_timeout: Duration,
    connection_label: String,
    _ssh_tunnel: Option<SshTunnel>,
}

#[derive(Debug, Clone, Copy)]
pub enum ManagerSignal {
    UnitNew,
    UnitRemoved,
}

impl SystemdManager {
    pub async fn connect_with_config(config: &ConnectionConfig) -> Result<Self> {
        let bus_kind = bus_kind_from_selection(config.bus);

        if config.ssh.is_some() {
            return Self::connect_via_ssh(bus_kind, config).await;
        }

        if let Some(socket_path) = config.dbus_socket.as_deref() {
            return Self::connect_via_unix_socket(bus_kind, socket_path, config.connect_timeout)
                .await;
        }

        match config.bus {
            BusSelection::Auto => Self::connect_auto(config.connect_timeout).await,
            BusSelection::System => Self::connect(BusKind::System, config.connect_timeout).await,
            BusSelection::Session => Self::connect(BusKind::Session, config.connect_timeout).await,
        }
    }

    pub async fn connect_auto(timeout: Duration) -> Result<Self> {
        match Self::connect(BusKind::System, timeout).await {
            Ok(manager) => Ok(manager),
            Err(system_err) => {
                let session = Self::connect(BusKind::Session, timeout).await;
                match session {
                    Ok(manager) => Ok(manager),
                    Err(session_err) => Err(anyhow::anyhow!(
                        "failed to connect system bus ({system_err}) and session bus ({session_err})"
                    )),
                }
            }
        }
    }

    pub async fn connect(bus_kind: BusKind, timeout: Duration) -> Result<Self> {
        let conn = tokio::time::timeout(timeout, async {
            match bus_kind {
                BusKind::System => Connection::system().await,
                BusKind::Session => Connection::session().await,
            }
        })
        .await
        .with_context(|| format!("timeout connecting to {} bus", bus_kind.as_str()))??;

        Ok(Self {
            conn,
            bus_kind,
            ssh_config: None,
            command_timeout: timeout,
            connection_label: connection_label_for(bus_kind, None),
            _ssh_tunnel: None,
        })
    }

    async fn connect_via_ssh(bus_kind: BusKind, config: &ConnectionConfig) -> Result<Self> {
        let ssh_config = config
            .ssh
            .as_ref()
            .context("missing SSH config for ssh connection mode")?;
        let socket_path = match config.dbus_socket.as_deref() {
            Some(socket_path) => socket_path,
            None => match bus_kind {
                BusKind::System => "/var/run/dbus/system_bus_socket",
                BusKind::Session => {
                    return Err(anyhow::anyhow!(
                        "remote session bus over ssh requires an explicit dbus socket path; pass --dbus-socket (for example /run/user/<uid>/bus)"
                    ));
                }
            },
        };
        let ssh_tunnel = SshTunnel::open(ssh_config, socket_path, config.connect_timeout).await?;
        let dbus_address = ssh_tunnel.dbus_tcp_address();
        let conn = tokio::time::timeout(
            config.connect_timeout,
            ConnectionBuilder::address(dbus_address.as_str())
                .context("invalid remote dbus address")?
                .build(),
        )
        .await
        .with_context(|| format!("timeout connecting to remote {bus_kind:?} bus via ssh"))??;

        Ok(Self {
            conn,
            bus_kind,
            ssh_config: Some(ssh_config.clone()),
            command_timeout: config.connect_timeout,
            connection_label: connection_label_for(bus_kind, Some(&ssh_config.host)),
            _ssh_tunnel: Some(ssh_tunnel),
        })
    }

    async fn connect_via_unix_socket(
        bus_kind: BusKind,
        socket_path: &str,
        timeout: Duration,
    ) -> Result<Self> {
        let dbus_address = format!("unix:path={socket_path}");
        let conn = tokio::time::timeout(
            timeout,
            ConnectionBuilder::address(dbus_address.as_str())
                .context("invalid local dbus unix socket address")?
                .build(),
        )
        .await
        .with_context(|| format!("timeout connecting to local unix socket {socket_path}"))??;

        Ok(Self {
            conn,
            bus_kind,
            ssh_config: None,
            command_timeout: timeout,
            connection_label: connection_label_for(bus_kind, None),
            _ssh_tunnel: None,
        })
    }

    pub fn connection_label(&self) -> &str {
        &self.connection_label
    }

    async fn proxy(&self) -> Result<zbus::Proxy<'_>> {
        zbus::Proxy::new(
            &self.conn,
            "org.freedesktop.systemd1",
            "/org/freedesktop/systemd1",
            "org.freedesktop.systemd1.Manager",
        )
        .await
        .context("failed to build systemd manager proxy")
    }

    pub async fn list_units(&self) -> Result<Vec<SystemdUnit>> {
        let rows: Vec<ListUnitsRow> = self
            .proxy()
            .await?
            .call("ListUnits", &())
            .await
            .context("ListUnits call failed")?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    name,
                    description,
                    load_state,
                    active_state,
                    sub_state,
                    follows,
                    unit_path,
                    _job_id,
                    _job_type,
                    _job_path,
                )| SystemdUnit {
                    name,
                    description,
                    load_state: UnitLoadState::from_raw(&load_state),
                    active_state: UnitActiveState::from_raw(&active_state),
                    sub_state,
                    follows,
                    path: unit_path.to_string(),
                },
            )
            .collect())
    }

    pub async fn start_unit(&self, unit_name: &str) -> Result<()> {
        let _: OwnedObjectPath = self
            .proxy()
            .await?
            .call("StartUnit", &(unit_name, "replace"))
            .await
            .with_context(|| format!("StartUnit failed for {unit_name}"))?;
        Ok(())
    }

    pub async fn stop_unit(&self, unit_name: &str) -> Result<()> {
        let _: OwnedObjectPath = self
            .proxy()
            .await?
            .call("StopUnit", &(unit_name, "replace"))
            .await
            .with_context(|| format!("StopUnit failed for {unit_name}"))?;
        Ok(())
    }

    pub async fn restart_unit(&self, unit_name: &str) -> Result<()> {
        let _: OwnedObjectPath = self
            .proxy()
            .await?
            .call("RestartUnit", &(unit_name, "replace"))
            .await
            .with_context(|| format!("RestartUnit failed for {unit_name}"))?;
        Ok(())
    }

    pub async fn reload_unit(&self, unit_name: &str) -> Result<()> {
        let _: OwnedObjectPath = self
            .proxy()
            .await?
            .call("ReloadUnit", &(unit_name, "replace"))
            .await
            .with_context(|| format!("ReloadUnit failed for {unit_name}"))?;
        Ok(())
    }

    pub async fn unit_logs(&self, unit_name: &str, limit: usize) -> Result<Vec<String>> {
        let args = self.journalctl_args(unit_name, limit);
        let output = match &self.ssh_config {
            Some(ssh_config) => self.run_remote_command(ssh_config, "journalctl", &args).await?,
            None => self.run_local_command("journalctl", &args).await?,
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let error_message = if stderr.is_empty() {
                format!("journalctl failed for {unit_name} with status {}", output.status)
            } else {
                format!("journalctl failed for {unit_name}: {stderr}")
            };
            return Err(anyhow::anyhow!(error_message));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(ToOwned::to_owned).collect())
    }

    pub async fn subscribe_unit_signals(&self) -> Result<mpsc::Receiver<ManagerSignal>> {
        let proxy = self.proxy().await?;
        let conn_for_unsubscribe = self.conn.clone();
        let _: () = proxy
            .call("Subscribe", &())
            .await
            .context("failed to subscribe manager for unit signals")?;
        let mut new_stream = proxy
            .receive_signal("UnitNew")
            .await
            .context("failed to subscribe UnitNew")?;
        let mut removed_stream = proxy
            .receive_signal("UnitRemoved")
            .await
            .context("failed to subscribe UnitRemoved")?;

        let (tx, rx) = mpsc::channel(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = new_stream.next() => {
                        if msg.is_none() {
                            break;
                        }
                        match tx.try_send(ManagerSignal::UnitNew) {
                            Ok(()) | Err(TrySendError::Full(_)) => {}
                            Err(TrySendError::Closed(_)) => break,
                        }
                    }
                    msg = removed_stream.next() => {
                        if msg.is_none() {
                            break;
                        }
                        match tx.try_send(ManagerSignal::UnitRemoved) {
                            Ok(()) | Err(TrySendError::Full(_)) => {}
                            Err(TrySendError::Closed(_)) => break,
                        }
                    }
                }
            }

            if let Ok(proxy) = zbus::Proxy::new(
                &conn_for_unsubscribe,
                "org.freedesktop.systemd1",
                "/org/freedesktop/systemd1",
                "org.freedesktop.systemd1.Manager",
            )
            .await
            {
                let _ = proxy.call::<_, _, ()>("Unsubscribe", &()).await;
            }
        });
        Ok(rx)
    }

    fn journalctl_args(&self, unit_name: &str, limit: usize) -> Vec<String> {
        let mut args = Vec::new();
        if self.bus_kind == BusKind::Session {
            args.push("--user".to_string());
        }
        args.extend([
            "-u".to_string(),
            unit_name.to_string(),
            "-n".to_string(),
            limit.to_string(),
            "--no-pager".to_string(),
            "-o".to_string(),
            "short-iso".to_string(),
        ]);
        args
    }

    async fn run_local_command(
        &self,
        program: &str,
        args: &[String],
    ) -> Result<std::process::Output> {
        let mut command = Command::new(program);
        command.args(args).stdin(Stdio::null());
        tokio::time::timeout(self.command_timeout, command.output())
            .await
            .with_context(|| format!("timeout running {program}"))?
            .with_context(|| format!("failed to run {program}"))
    }

    async fn run_remote_command(
        &self,
        ssh_config: &SshConfig,
        program: &str,
        args: &[String],
    ) -> Result<std::process::Output> {
        let destination = match &ssh_config.user {
            Some(user) => format!("{user}@{}", ssh_config.host),
            None => ssh_config.host.clone(),
        };

        let mut command = Command::new("ssh");
        command
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-p")
            .arg(ssh_config.port.to_string());
        if let Some(key_path) = &ssh_config.key_path {
            command.arg("-i").arg(key_path);
        }
        command
            .arg(&destination)
            .arg(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        tokio::time::timeout(self.command_timeout, command.output())
            .await
            .with_context(|| format!("timeout running remote {program} via ssh"))?
            .with_context(|| format!("failed to run remote {program} via ssh"))
    }
}

fn connection_label_for(bus_kind: BusKind, ssh_host: Option<&str>) -> String {
    match ssh_host {
        Some(host) => format!("remote:ssh({host})"),
        None => format!("local:{}", bus_kind.as_str()),
    }
}

fn bus_kind_from_selection(selection: BusSelection) -> BusKind {
    match selection {
        BusSelection::Auto | BusSelection::System => BusKind::System,
        BusSelection::Session => BusKind::Session,
    }
}

#[cfg(test)]
mod tests {
    use super::{bus_kind_from_selection, connection_label_for};
    use crate::bus::{BusKind, BusSelection};

    #[test]
    fn connection_label_for_local_bus() {
        assert_eq!(connection_label_for(BusKind::System, None), "local:system");
        assert_eq!(
            connection_label_for(BusKind::Session, None),
            "local:session"
        );
    }

    #[test]
    fn connection_label_for_remote_ssh() {
        assert_eq!(
            connection_label_for(BusKind::System, Some("srv.example")),
            "remote:ssh(srv.example)"
        );
    }

    #[test]
    fn bus_kind_from_selection_defaults_auto_to_system() {
        assert_eq!(bus_kind_from_selection(BusSelection::Auto), BusKind::System);
        assert_eq!(
            bus_kind_from_selection(BusSelection::System),
            BusKind::System
        );
        assert_eq!(
            bus_kind_from_selection(BusSelection::Session),
            BusKind::Session
        );
    }
}

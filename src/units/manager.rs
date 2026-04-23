use crate::bus::{BusKind, BusSelection, ConnectionConfig, SshTunnel};
use crate::units::structs::{SystemdUnit, UnitActiveState, UnitLoadState};
use anyhow::{Context, Result};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, ConnectionBuilder};

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

    pub async fn subscribe_unit_signals(&self) -> Result<mpsc::UnboundedReceiver<ManagerSignal>> {
        let proxy = self.proxy().await?;
        let mut new_stream = proxy
            .receive_signal("UnitNew")
            .await
            .context("failed to subscribe UnitNew")?;
        let mut removed_stream = proxy
            .receive_signal("UnitRemoved")
            .await
            .context("failed to subscribe UnitRemoved")?;

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = new_stream.next() => {
                        if msg.is_none() {
                            break;
                        }
                        if tx.send(ManagerSignal::UnitNew).is_err() {
                            break;
                        }
                    }
                    msg = removed_stream.next() => {
                        if msg.is_none() {
                            break;
                        }
                        if tx.send(ManagerSignal::UnitRemoved).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Ok(rx)
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

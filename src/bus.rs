use anyhow::{Context, Result, bail};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::time::{Instant, sleep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusKind {
    System,
    Session,
}

impl BusKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Session => "session",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusSelection {
    Auto,
    System,
    Session,
}

#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub user: Option<String>,
    pub port: u16,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub bus: BusSelection,
    pub ssh: Option<SshConfig>,
    pub dbus_socket: Option<String>,
    pub connect_timeout: Duration,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            bus: BusSelection::Auto,
            ssh: None,
            dbus_socket: None,
            connect_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Debug)]
pub struct SshTunnel {
    child: Child,
    pub local_port: u16,
}

impl SshTunnel {
    pub async fn open(
        config: &SshConfig,
        remote_socket: &str,
        connect_timeout: Duration,
    ) -> Result<Self> {
        let local_port = reserve_local_port()?;
        let destination = match &config.user {
            Some(user) => format!("{user}@{}", config.host),
            None => config.host.clone(),
        };
        let forward_spec = format!("{local_port}:{remote_socket}");

        let mut cmd = Command::new("ssh");
        cmd.arg("-N")
            .arg("-o")
            .arg("ExitOnForwardFailure=yes")
            .arg("-o")
            .arg("ServerAliveInterval=30")
            .arg("-o")
            .arg("ServerAliveCountMax=3")
            .arg("-p")
            .arg(config.port.to_string());
        if let Some(key_path) = &config.key_path {
            cmd.arg("-i").arg(key_path);
        }
        cmd.arg("-L")
            .arg(forward_spec)
            .arg(destination)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = cmd.spawn().context("failed to spawn ssh process")?;

        let deadline = Instant::now() + connect_timeout;
        loop {
            if let Some(status) = child
                .try_wait()
                .context("failed to check ssh process state")?
            {
                bail!("ssh tunnel exited early with status: {status}");
            }

            if TcpStream::connect(("127.0.0.1", local_port)).await.is_ok() {
                break;
            }

            if Instant::now() >= deadline {
                child.start_kill().ok();
                bail!("timeout waiting for ssh tunnel to be ready");
            }

            sleep(Duration::from_millis(75)).await;
        }

        Ok(Self { child, local_port })
    }

    pub fn dbus_tcp_address(&self) -> String {
        format!("tcp:host=127.0.0.1,port={}", self.local_port)
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

fn reserve_local_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to allocate local tcp port")?;
    let port = listener
        .local_addr()
        .context("failed to read local listener address")?
        .port();
    Ok(port)
}

#[cfg(test)]
mod tests {
    use super::{BusKind, BusSelection, ConnectionConfig, reserve_local_port};

    #[test]
    fn bus_kind_as_str() {
        assert_eq!(BusKind::System.as_str(), "system");
        assert_eq!(BusKind::Session.as_str(), "session");
    }

    #[test]
    fn connection_config_default_is_safe() {
        let cfg = ConnectionConfig::default();
        assert_eq!(cfg.bus, BusSelection::Auto);
        assert!(cfg.ssh.is_none());
        assert!(cfg.dbus_socket.is_none());
        assert_eq!(cfg.connect_timeout.as_secs(), 5);
    }

    #[test]
    fn reserve_local_port_returns_non_zero_port() {
        let port = reserve_local_port().expect("must allocate local port");
        assert!(port > 0);
    }
}

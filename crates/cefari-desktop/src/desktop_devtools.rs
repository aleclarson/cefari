use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

use anyhow::{Context, Result};

pub(crate) const CEFARI_DEV_MODE_ENV: &str = "CEFARI_DEV_MODE";
pub(crate) const CEFARI_DEVTOOLS_PORT_ENV: &str = "CEFARI_DEVTOOLS_PORT";
pub(crate) const DEVTOOLS_LOOPBACK_HOST: Ipv4Addr = Ipv4Addr::LOCALHOST;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct DevtoolsPort(u16);

impl DevtoolsPort {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        value
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .map(Self)
    }

    pub(crate) fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum DevtoolsEndpointRole {
    PublicMux,
    PrivateCef,
    PrivateDenoDaemon,
    PrivateDenoWorker,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct DevtoolsEndpoint {
    pub(crate) role: DevtoolsEndpointRole,
    pub(crate) host: Ipv4Addr,
    pub(crate) port: DevtoolsPort,
}

impl DevtoolsEndpoint {
    pub(crate) fn loopback(role: DevtoolsEndpointRole, port: DevtoolsPort) -> Self {
        Self {
            role,
            host: DEVTOOLS_LOOPBACK_HOST,
            port,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn socket_addr(self) -> SocketAddrV4 {
        SocketAddrV4::new(self.host, self.port.get())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct DevtoolsSessionConfig {
    pub(crate) public_endpoint: DevtoolsEndpoint,
}

impl DevtoolsSessionConfig {
    pub(crate) fn from_environment() -> Option<Self> {
        if !dev_mode_enabled() {
            return None;
        }
        let port = std::env::var(CEFARI_DEVTOOLS_PORT_ENV).ok()?;
        let port = DevtoolsPort::parse(&port)?;
        Some(Self {
            public_endpoint: DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PublicMux, port),
        })
    }
}

pub(crate) fn dev_mode_enabled() -> bool {
    std::env::var(CEFARI_DEV_MODE_ENV).as_deref() == Ok("1")
}

#[allow(dead_code)]
pub(crate) fn allocate_private_loopback_endpoint(
    role: DevtoolsEndpointRole,
) -> Result<DevtoolsEndpoint> {
    let listener = TcpListener::bind(SocketAddrV4::new(DEVTOOLS_LOOPBACK_HOST, 0))
        .context("failed to allocate private DevTools loopback port")?;
    let port = listener
        .local_addr()
        .context("failed to read allocated DevTools loopback port")?
        .port();
    Ok(DevtoolsEndpoint::loopback(role, DevtoolsPort(port)))
}

#[cfg(test)]
mod tests {
    use super::{
        allocate_private_loopback_endpoint, DevtoolsEndpoint, DevtoolsEndpointRole, DevtoolsPort,
        DEVTOOLS_LOOPBACK_HOST,
    };

    #[test]
    fn parses_nonzero_devtools_ports() {
        assert_eq!(DevtoolsPort::parse("9222"), Some(DevtoolsPort(9222)));
        assert_eq!(DevtoolsPort::parse("0"), None);
        assert_eq!(DevtoolsPort::parse("not-a-port"), None);
    }

    #[test]
    fn builds_loopback_endpoint_for_role() {
        let endpoint =
            DevtoolsEndpoint::loopback(DevtoolsEndpointRole::PublicMux, DevtoolsPort(9222));

        assert_eq!(endpoint.role, DevtoolsEndpointRole::PublicMux);
        assert_eq!(endpoint.socket_addr().ip(), &DEVTOOLS_LOOPBACK_HOST);
        assert_eq!(endpoint.socket_addr().port(), 9222);
    }

    #[test]
    fn allocates_private_loopback_endpoint() {
        let endpoint =
            allocate_private_loopback_endpoint(DevtoolsEndpointRole::PrivateCef).unwrap();

        assert_eq!(endpoint.role, DevtoolsEndpointRole::PrivateCef);
        assert_eq!(endpoint.host, DEVTOOLS_LOOPBACK_HOST);
        assert_ne!(endpoint.port.get(), 0);
    }
}

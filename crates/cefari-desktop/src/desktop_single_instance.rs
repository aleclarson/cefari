use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use cefari_core::RuntimePaths;
use single_instance::SingleInstance;
use tracing::{debug, error, info};

use crate::event_loop::UserEvent;

const FORWARD_PORT_FILE: &str = "cefari-deep-link-forwarder.port";
const FORWARD_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
const FORWARD_RETRY_DELAY: Duration = Duration::from_millis(50);

pub(crate) enum InstanceStartup {
    Primary {
        instance: SingleInstance,
        startup_deep_links: Vec<String>,
    },
    Forwarded,
}

pub(crate) struct DeepLinkForwarder {
    port_file: PathBuf,
}

pub(crate) fn acquire_or_forward(
    paths: &RuntimePaths,
    deep_link_schemes: &[String],
    args: impl IntoIterator<Item = String>,
) -> Result<InstanceStartup> {
    fs::create_dir_all(&paths.cache_dir).with_context(|| {
        format!(
            "failed to create cache directory at {}",
            paths.cache_dir.display()
        )
    })?;

    let lock_path = paths.cache_dir.join("cefari.lock");
    let instance = SingleInstance::new(&lock_path.display().to_string()).with_context(|| {
        format!(
            "failed to create single-instance lock at {}",
            lock_path.display()
        )
    })?;

    if instance.is_single() {
        return Ok(InstanceStartup::Primary {
            instance,
            startup_deep_links: startup_deep_link_urls(args, deep_link_schemes),
        });
    }

    let startup_deep_links = startup_deep_link_urls(args, deep_link_schemes);
    if startup_deep_links.is_empty() {
        anyhow::bail!("another Cefari instance is already running");
    }
    forward_deep_links(&forward_port_file(paths), &startup_deep_links)?;
    Ok(InstanceStartup::Forwarded)
}

pub(crate) fn start_deep_link_forwarder(
    paths: &RuntimePaths,
    deep_link_schemes: &[String],
    event_proxy: tao::event_loop::EventLoopProxy<UserEvent>,
) -> Result<DeepLinkForwarder> {
    fs::create_dir_all(&paths.cache_dir).with_context(|| {
        format!(
            "failed to create cache directory at {}",
            paths.cache_dir.display()
        )
    })?;
    let listener =
        TcpListener::bind("127.0.0.1:0").context("failed to bind deep link forwarder")?;
    let port = listener
        .local_addr()
        .context("failed to read deep link forwarder address")?
        .port();
    let port_file = forward_port_file(paths);
    fs::write(&port_file, port.to_string()).with_context(|| {
        format!(
            "failed to write deep link forwarder port file at {}",
            port_file.display()
        )
    })?;

    let schemes = deep_link_schemes.to_vec();
    thread::spawn(move || run_deep_link_forwarder(listener, schemes, event_proxy));
    info!(port, "started deep link forwarder");
    Ok(DeepLinkForwarder { port_file })
}

impl Drop for DeepLinkForwarder {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.port_file);
    }
}

pub(crate) fn startup_deep_link_urls(
    args: impl IntoIterator<Item = String>,
    deep_link_schemes: &[String],
) -> Vec<String> {
    args.into_iter()
        .skip(1)
        .filter(|arg| is_configured_deep_link_url(arg, deep_link_schemes))
        .collect()
}

fn run_deep_link_forwarder(
    listener: TcpListener,
    deep_link_schemes: Vec<String>,
    event_proxy: tao::event_loop::EventLoopProxy<UserEvent>,
) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => receive_forwarded_deep_links(stream, &deep_link_schemes, &event_proxy),
            Err(error) => error!(%error, "failed to accept forwarded deep link connection"),
        }
    }
}

fn receive_forwarded_deep_links(
    stream: TcpStream,
    deep_link_schemes: &[String],
    event_proxy: &tao::event_loop::EventLoopProxy<UserEvent>,
) {
    for line in BufReader::new(stream).lines() {
        match line {
            Ok(url) if is_configured_deep_link_url(&url, deep_link_schemes) => {
                if event_proxy
                    .send_event(UserEvent::ForwardedDeepLink(url))
                    .is_err()
                {
                    error!("failed to send forwarded deep link to event loop");
                    return;
                }
            }
            Ok(url) => debug!(url, "ignored forwarded URL with unconfigured scheme"),
            Err(error) => {
                error!(%error, "failed to read forwarded deep link");
                return;
            }
        }
    }
}

fn forward_deep_links(port_file: &Path, urls: &[String]) -> Result<()> {
    let deadline = Instant::now() + FORWARD_CONNECT_TIMEOUT;
    loop {
        match try_forward_deep_links(port_file, urls) {
            Ok(()) => return Ok(()),
            Err(error) if Instant::now() < deadline => {
                debug!(%error, "retrying deep link forward");
                thread::sleep(FORWARD_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
}

fn try_forward_deep_links(port_file: &Path, urls: &[String]) -> Result<()> {
    let port = fs::read_to_string(port_file)
        .with_context(|| {
            format!(
                "failed to read deep link forwarder port at {}",
                port_file.display()
            )
        })?
        .trim()
        .parse::<u16>()
        .context("deep link forwarder port was invalid")?;
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .with_context(|| format!("failed to connect to deep link forwarder on port {port}"))?;
    for url in urls {
        writeln!(stream, "{url}").context("failed to forward deep link URL")?;
    }
    Ok(())
}

fn forward_port_file(paths: &RuntimePaths) -> PathBuf {
    paths.cache_dir.join(FORWARD_PORT_FILE)
}

fn is_configured_deep_link_url(url: &str, deep_link_schemes: &[String]) -> bool {
    url_scheme(url).is_some_and(|scheme| {
        deep_link_schemes
            .iter()
            .any(|configured| configured == scheme)
    })
}

fn url_scheme(url: &str) -> Option<&str> {
    let (scheme, _rest) = url.split_once(':')?;
    (!scheme.is_empty()).then_some(scheme)
}

#[cfg(test)]
mod tests {
    use super::{is_configured_deep_link_url, startup_deep_link_urls, url_scheme};

    #[test]
    fn startup_deep_link_urls_keeps_only_configured_schemes() {
        let args = vec![
            "app".to_owned(),
            "myapp://open".to_owned(),
            "https://example.test".to_owned(),
            "other://ignored".to_owned(),
            "--flag".to_owned(),
        ];
        let schemes = vec!["myapp".to_owned()];

        assert_eq!(startup_deep_link_urls(args, &schemes), vec!["myapp://open"]);
    }

    #[test]
    fn deep_link_url_detection_requires_exact_configured_scheme() {
        let schemes = vec!["myapp".to_owned()];

        assert!(is_configured_deep_link_url("myapp://open", &schemes));
        assert!(!is_configured_deep_link_url("MyApp://open", &schemes));
        assert!(!is_configured_deep_link_url("myapp-extra://open", &schemes));
        assert!(!is_configured_deep_link_url("not a url", &schemes));
        assert_eq!(url_scheme("myapp://open"), Some("myapp"));
    }
}

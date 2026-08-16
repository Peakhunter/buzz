//! Source-bound public relay origin resolution for application requests.

use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};

use axum::body::Body;
use axum::extract::{ConnectInfo, State};
use axum::http::{header::HOST, HeaderMap, Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

/// A normalized, bare public WebSocket origin (scheme plus authority).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RelayOrigin(String);

/// Marker inserted only for requests accepted through the configured Unix socket.
#[derive(Clone, Copy, Debug)]
pub struct UdsIngress;

impl RelayOrigin {
    /// Parse and normalize a `ws`/`wss` bare origin.
    pub fn parse(value: &str) -> Result<Self, String> {
        if value.trim() != value || value.is_empty() {
            return Err("invalid relay origin".to_string());
        }
        let parsed = url::Url::parse(value).map_err(|_| "invalid relay origin".to_string())?;
        if !matches!(parsed.scheme(), "ws" | "wss")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || !matches!(parsed.path(), "" | "/")
            || parsed.query().is_some()
            || parsed.fragment().is_some()
        {
            return Err("relay origin must be a bare ws/wss origin".to_string());
        }
        let authority = authority(&parsed)?;
        Ok(Self(format!("{}://{authority}", parsed.scheme())))
    }

    /// Return the normalized WebSocket origin.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Construct the exact HTTP(S) URL used by NIP-98.
    pub fn http_url(&self, path: &str) -> String {
        let scheme = if self.0.starts_with("wss://") {
            "https"
        } else {
            "http"
        };
        let authority = self
            .0
            .strip_prefix("wss://")
            .or_else(|| self.0.strip_prefix("ws://"))
            .unwrap_or(&self.0);
        format!("{scheme}://{authority}{path}")
    }
}

fn authority(url: &url::Url) -> Result<String, String> {
    let host = url
        .host()
        .ok_or_else(|| "relay origin requires a host".to_string())?;
    let host = match host {
        url::Host::Ipv6(addr) => format!("[{addr}]"),
        _ => host.to_string(),
    };
    Ok(match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    })
}

/// Resolve the public origin from the socket peer and request headers.
pub fn resolve_request_origin(
    peer: SocketAddr,
    headers: &HeaderMap,
    direct_origin: &RelayOrigin,
    accepted: &HashSet<RelayOrigin>,
    trusted_proxies: &HashSet<IpAddr>,
) -> Result<RelayOrigin, String> {
    let proto = single_header(headers, "x-forwarded-proto")?;
    let forwarded_host = single_header(headers, "x-forwarded-host")?;
    let trusted = trusted_proxies.contains(&peer.ip());

    let candidate = if trusted {
        let proto = proto.ok_or_else(|| "missing forwarded origin".to_string())?;
        let host = forwarded_host.ok_or_else(|| "missing forwarded origin".to_string())?;
        let ws_scheme = match proto {
            "http" => "ws",
            "https" => "wss",
            _ => return Err("invalid forwarded scheme".to_string()),
        };
        request_origin(ws_scheme, host)?
    } else {
        if proto.is_some() || forwarded_host.is_some() {
            return Err("forwarded headers from untrusted peer".to_string());
        }
        let host =
            single_header(headers, HOST.as_str())?.ok_or_else(|| "missing host".to_string())?;
        let scheme = if direct_origin.as_str().starts_with("wss://") {
            "wss"
        } else {
            "ws"
        };
        request_origin(scheme, host)?
    };

    accepted
        .contains(&candidate)
        .then_some(candidate)
        .ok_or_else(|| "unaccepted relay origin".to_string())
}

/// Resolve an origin for the trusted Unix-socket listener.
///
/// Without forwarding metadata this preserves direct-origin behavior. A local
/// service-mesh proxy may instead supply both forwarded fields to select another
/// explicitly accepted origin. Partial or malformed metadata fails closed.
pub fn resolve_uds_request_origin(
    headers: &HeaderMap,
    direct_origin: &RelayOrigin,
    accepted: &HashSet<RelayOrigin>,
) -> Result<RelayOrigin, String> {
    let proto = single_header(headers, "x-forwarded-proto")?;
    let forwarded_host = single_header(headers, "x-forwarded-host")?;
    let candidate = match (proto, forwarded_host) {
        (None, None) => {
            let host =
                single_header(headers, HOST.as_str())?.ok_or_else(|| "missing host".to_string())?;
            let scheme = if direct_origin.as_str().starts_with("wss://") {
                "wss"
            } else {
                "ws"
            };
            request_origin(scheme, host)?
        }
        (Some(proto), Some(host)) => {
            let scheme = match proto {
                "http" => "ws",
                "https" => "wss",
                _ => return Err("invalid forwarded scheme".to_string()),
            };
            request_origin(scheme, host)?
        }
        _ => return Err("incomplete forwarded origin".to_string()),
    };
    accepted
        .contains(&candidate)
        .then_some(candidate)
        .ok_or_else(|| "unaccepted relay origin".to_string())
}

fn request_origin(scheme: &str, authority: &str) -> Result<RelayOrigin, String> {
    if authority.contains(['/', '?', '#', '@', '\\']) {
        return Err("malformed authority".to_string());
    }
    RelayOrigin::parse(&format!("{scheme}://{authority}"))
}

fn single_header<'a>(headers: &'a HeaderMap, name: &str) -> Result<Option<&'a str>, String> {
    let mut values = headers.get_all(name).iter();
    let Some(value) = values.next() else {
        return Ok(None);
    };
    if values.next().is_some() {
        return Err("duplicate header".to_string());
    }
    let value = value.to_str().map_err(|_| "malformed header".to_string())?;
    if value.is_empty() || value.contains(',') || value.trim() != value {
        return Err("malformed header".to_string());
    }
    Ok(Some(value))
}

/// Resolve and attach the origin once, before any application route executes.
pub async fn resolve_origin_middleware(
    State(state): State<std::sync::Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|value| value.0);
    // Existing in-process router tests do not have a real listener. Preserve
    // their direct-TCP behavior without affecting production ingress checks.
    #[cfg(test)]
    let peer = peer.or_else(|| {
        request
            .extensions()
            .get::<UdsIngress>()
            .is_none()
            .then(|| SocketAddr::from(([127, 0, 0, 1], 0)))
    });
    let origin = if let Some(peer) = peer {
        resolve_request_origin(
            peer,
            request.headers(),
            &state.config.relay_origin,
            &state.config.accepted_relay_origins,
            &state.config.trusted_proxy_ips,
        )
    } else if request.extensions().get::<UdsIngress>().is_some() {
        resolve_uds_request_origin(
            request.headers(),
            &state.config.relay_origin,
            &state.config.accepted_relay_origins,
        )
    } else {
        Err("missing ingress identity".to_string())
    };
    match origin {
        Ok(origin) => {
            request.extensions_mut().insert(origin);
            next.run(request).await
        }
        Err(_) => (StatusCode::BAD_REQUEST, "invalid request origin").into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn origin(value: &str) -> RelayOrigin {
        RelayOrigin::parse(value).expect("valid test origin")
    }

    fn accepted(values: &[&str]) -> HashSet<RelayOrigin> {
        values.iter().map(|value| origin(value)).collect()
    }

    #[test]
    fn parser_normalizes_only_bare_ws_origins() {
        assert_eq!(
            origin("wss://Example.COM:8443/").as_str(),
            "wss://example.com:8443"
        );
        assert_eq!(
            origin("ws://buzz.peakhunter.com:3000/").as_str(),
            "ws://buzz.peakhunter.com:3000"
        );
        for invalid in [
            "https://example.com",
            "ws://user@example.com",
            "ws://example.com/path",
            "ws://example.com/?query",
            "ws://example.com/#fragment",
            "ws://",
            "example.com",
        ] {
            assert!(RelayOrigin::parse(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn trusted_forwarded_tls_resolves_exact_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("buzz.peakhunter.com:3000"));
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("buzz.peakhunter.com:8443"),
        );
        let trusted = HashSet::from(["10.0.0.1".parse().unwrap()]);
        let resolved = resolve_request_origin(
            "10.0.0.1:1234".parse().unwrap(),
            &headers,
            &origin("ws://buzz.peakhunter.com:3000"),
            &accepted(&[
                "ws://buzz.peakhunter.com:3000",
                "wss://buzz.peakhunter.com:8443",
            ]),
            &trusted,
        )
        .unwrap();
        assert_eq!(resolved.as_str(), "wss://buzz.peakhunter.com:8443");
        assert_eq!(headers[HOST], "buzz.peakhunter.com:3000");
    }

    #[test]
    fn direct_canonical_resolves_and_never_infers_tls_from_port() {
        let direct = origin("ws://buzz.peakhunter.com:3000");
        let accepted = accepted(&[
            "ws://buzz.peakhunter.com:3000",
            "wss://buzz.peakhunter.com:8443",
        ]);
        let trusted = HashSet::new();
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("buzz.peakhunter.com:3000"));
        assert_eq!(
            resolve_request_origin(
                "192.0.2.1:1".parse().unwrap(),
                &headers,
                &direct,
                &accepted,
                &trusted
            )
            .unwrap(),
            direct
        );
        headers.insert(HOST, HeaderValue::from_static("buzz.peakhunter.com:8443"));
        assert!(resolve_request_origin(
            "192.0.2.1:1".parse().unwrap(),
            &headers,
            &direct,
            &accepted,
            &trusted
        )
        .is_err());
    }

    #[test]
    fn rejects_untrusted_forwarding_and_bad_trusted_forwarding() {
        let direct = origin("ws://buzz.peakhunter.com:3000");
        let accepted = accepted(&[direct.as_str(), "wss://buzz.peakhunter.com:8443"]);
        let trusted = HashSet::from(["10.0.0.1".parse().unwrap()]);
        let peer: SocketAddr = "10.0.0.1:1".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("buzz.peakhunter.com:3000"));
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert!(resolve_request_origin(peer, &headers, &direct, &accepted, &trusted).is_err());
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("unknown.example"),
        );
        assert!(resolve_request_origin(peer, &headers, &direct, &accepted, &trusted).is_err());
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https,http"));
        assert!(resolve_request_origin(peer, &headers, &direct, &accepted, &trusted).is_err());
        headers.insert("x-forwarded-proto", HeaderValue::from_static("ftp"));
        assert!(resolve_request_origin(peer, &headers, &direct, &accepted, &trusted).is_err());
        headers.append("x-forwarded-proto", HeaderValue::from_static("https"));
        assert!(resolve_request_origin(peer, &headers, &direct, &accepted, &trusted).is_err());

        let untrusted: SocketAddr = "192.0.2.1:1".parse().unwrap();
        assert!(resolve_request_origin(untrusted, &headers, &direct, &accepted, &trusted).is_err());
    }

    #[test]
    fn uds_supports_direct_and_complete_forwarded_origins_only() {
        let direct = origin("ws://buzz.peakhunter.com:3000");
        let accepted = accepted(&[
            "ws://buzz.peakhunter.com:3000",
            "wss://buzz.peakhunter.com:8443",
        ]);
        let mut headers = HeaderMap::new();
        headers.insert(HOST, HeaderValue::from_static("buzz.peakhunter.com:3000"));
        assert_eq!(
            resolve_uds_request_origin(&headers, &direct, &accepted).unwrap(),
            direct
        );

        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));
        assert!(resolve_uds_request_origin(&headers, &direct, &accepted).is_err());
        headers.insert(
            "x-forwarded-host",
            HeaderValue::from_static("buzz.peakhunter.com:8443"),
        );
        assert_eq!(
            resolve_uds_request_origin(&headers, &direct, &accepted)
                .unwrap()
                .as_str(),
            "wss://buzz.peakhunter.com:8443"
        );
    }
}

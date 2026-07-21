use crate::engine::{AdblockEngine, PROXY_ADDRESS};
use std::{io, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use url::Url;

const MAX_HEADER_BYTES: usize = 64 * 1024;

pub fn spawn(engine: AdblockEngine) {
    tauri::async_runtime::spawn(async move {
        let listener = match TcpListener::bind(PROXY_ADDRESS).await {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("SinkHole proxy could not bind to {PROXY_ADDRESS}: {error}");
                engine.set_proxy_running(false);
                return;
            }
        };

        engine.set_proxy_running(true);
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    let engine = engine.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = handle_connection(stream, engine).await {
                            eprintln!("SinkHole proxy connection failed: {error}");
                        }
                    });
                }
                Err(error) => {
                    eprintln!("SinkHole proxy accept failed: {error}");
                }
            }
        }
    });
}

async fn handle_connection(mut client: TcpStream, engine: AdblockEngine) -> io::Result<()> {
    let request = read_request_head(&mut client).await?;
    let Some(parsed) = ParsedRequest::from_bytes(&request) else {
        return write_response(&mut client, "400 Bad Request").await;
    };

    if engine.check_host(&parsed.host) {
        engine.record_block();
        return write_response(&mut client, "204 No Content").await;
    }

    if parsed.method.eq_ignore_ascii_case("CONNECT") {
        return tunnel_https(client, &parsed.upstream).await;
    }

    forward_http(client, request, parsed).await
}

async fn read_request_head(client: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut request = Vec::with_capacity(4096);
    let mut chunk = [0_u8; 4096];

    loop {
        let bytes_read = client.read(&mut chunk).await?;
        if bytes_read == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..bytes_read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if request.len() > MAX_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "request headers exceed 64 KiB",
            ));
        }
    }

    Ok(request)
}

async fn tunnel_https(mut client: TcpStream, upstream_address: &str) -> io::Result<()> {
    let mut upstream = connect_with_timeout(upstream_address).await?;
    client
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;
    tokio::io::copy_bidirectional(&mut client, &mut upstream).await?;
    Ok(())
}

async fn forward_http(
    mut client: TcpStream,
    request: Vec<u8>,
    parsed: ParsedRequest,
) -> io::Result<()> {
    let mut upstream = connect_with_timeout(&parsed.upstream).await?;
    let outbound = rewrite_proxy_request(&request, &parsed)?;
    upstream.write_all(&outbound).await?;
    tokio::io::copy_bidirectional(&mut client, &mut upstream).await?;
    Ok(())
}

async fn connect_with_timeout(address: &str) -> io::Result<TcpStream> {
    timeout(Duration::from_secs(10), TcpStream::connect(address))
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "upstream connection timed out"))?
}

async fn write_response(client: &mut TcpStream, status: &str) -> io::Result<()> {
    client
        .write_all(
            format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
}

#[derive(Debug)]
struct ParsedRequest {
    method: String,
    version: String,
    host: String,
    upstream: String,
    origin_target: String,
}

impl ParsedRequest {
    fn from_bytes(request: &[u8]) -> Option<Self> {
        let header_end = find_header_end(request)?;
        let header = std::str::from_utf8(&request[..header_end]).ok()?;
        let mut lines = header.split("\r\n");
        let mut request_parts = lines.next()?.split_whitespace();
        let method = request_parts.next()?.to_owned();
        let target = request_parts.next()?;
        let version = request_parts.next()?.to_owned();

        if method.eq_ignore_ascii_case("CONNECT") {
            let host = host_without_port(target)?;
            let upstream = if target.contains(':') {
                target.to_owned()
            } else {
                format!("{target}:443")
            };
            return Some(Self {
                method,
                version,
                host,
                upstream,
                origin_target: target.to_owned(),
            });
        }

        if let Ok(url) = Url::parse(target) {
            let host = url.host_str()?.to_owned();
            let port = url.port_or_known_default()?;
            let upstream = format!("{host}:{port}");
            let mut origin_target = url.path().to_owned();
            if origin_target.is_empty() {
                origin_target.push('/');
            }
            if let Some(query) = url.query() {
                origin_target.push('?');
                origin_target.push_str(query);
            }
            return Some(Self {
                method,
                version,
                host,
                upstream,
                origin_target,
            });
        }

        let host_header = lines.find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("host").then_some(value.trim())
        })?;
        let host = host_without_port(host_header)?;
        let upstream = if host_header.rsplit_once(':').is_some() {
            host_header.to_owned()
        } else {
            format!("{host_header}:80")
        };
        Some(Self {
            method,
            version,
            host,
            upstream,
            origin_target: target.to_owned(),
        })
    }
}

fn rewrite_proxy_request(request: &[u8], parsed: &ParsedRequest) -> io::Result<Vec<u8>> {
    let header_end = find_header_end(request)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "incomplete request"))?;
    let header = std::str::from_utf8(&request[..header_end])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "non-UTF-8 request headers"))?;

    let mut outbound = format!(
        "{} {} {}\r\n",
        parsed.method, parsed.origin_target, parsed.version
    );
    for line in header.split("\r\n").skip(1) {
        if line.is_empty()
            || line.split_once(':').is_some_and(|(name, _)| {
                matches_ignore_ascii_case(name, "connection", "proxy-connection")
            })
        {
            continue;
        }
        outbound.push_str(line);
        outbound.push_str("\r\n");
    }
    outbound.push_str("Connection: close\r\n\r\n");

    let mut bytes = outbound.into_bytes();
    bytes.extend_from_slice(&request[header_end + 4..]);
    Ok(bytes)
}

fn matches_ignore_ascii_case(value: &str, first: &str, second: &str) -> bool {
    value.eq_ignore_ascii_case(first) || value.eq_ignore_ascii_case(second)
}

fn find_header_end(request: &[u8]) -> Option<usize> {
    request.windows(4).position(|window| window == b"\r\n\r\n")
}

fn host_without_port(authority: &str) -> Option<String> {
    let authority = authority.trim();
    if authority.starts_with('[') {
        return authority
            .strip_prefix('[')?
            .split_once(']')
            .map(|(host, _)| host.to_owned());
    }
    Some(
        authority
            .rsplit_once(':')
            .filter(|(_, port)| port.parse::<u16>().is_ok())
            .map_or(authority, |(host, _)| host)
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_absolute_http_proxy_request() {
        let request = b"GET http://ads.example.com:8080/banner.js?q=1 HTTP/1.1\r\nHost: ads.example.com:8080\r\n\r\n";
        let parsed = ParsedRequest::from_bytes(request).expect("request should parse");
        assert_eq!(parsed.host, "ads.example.com");
        assert_eq!(parsed.upstream, "ads.example.com:8080");
        assert_eq!(parsed.origin_target, "/banner.js?q=1");
    }

    #[test]
    fn parses_connect_request() {
        let request = b"CONNECT ads.example.com:443 HTTP/1.1\r\nHost: ads.example.com:443\r\n\r\n";
        let parsed = ParsedRequest::from_bytes(request).expect("request should parse");
        assert_eq!(parsed.host, "ads.example.com");
        assert_eq!(parsed.upstream, "ads.example.com:443");
    }

    #[test]
    fn sinkholes_blocked_request_over_tcp_and_counts_it() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime should build");

        runtime.block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .expect("test listener should bind");
            let address = listener.local_addr().expect("listener should have an address");
            let engine = AdblockEngine::new(true, &[]);
            let server_engine = engine.clone();
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.expect("client should connect");
                handle_connection(stream, server_engine)
                    .await
                    .expect("request should be handled");
            });

            let mut client = TcpStream::connect(address)
                .await
                .expect("proxy client should connect");
            client
                .write_all(
                    b"GET http://ads.doubleclick.net/pagead.js HTTP/1.1\r\nHost: ads.doubleclick.net\r\nConnection: close\r\n\r\n",
                )
                .await
                .expect("request should write");
            let mut response = Vec::new();
            client
                .read_to_end(&mut response)
                .await
                .expect("response should read");
            server.await.expect("proxy task should finish");

            let response = String::from_utf8(response).expect("response should be UTF-8");
            assert!(response.starts_with("HTTP/1.1 204 No Content"));
            assert_eq!(engine.stats().total_blocked, 1);
        });
    }
}

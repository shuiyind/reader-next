use reqwest::{redirect::Policy, Client};
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::time::Duration;
use tokio::net::lookup_host;
use url::Url;

pub const MAX_REMOTE_RESPONSE_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_PROXY_RESPONSE_BYTES: usize = 25 * 1024 * 1024;
pub const MAX_COVER_RESPONSE_BYTES: usize = 5 * 1024 * 1024;

pub fn parse_public_http_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw.trim()).map_err(|error| format!("无效 URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("仅允许 HTTP/HTTPS URL".to_string());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL 不允许包含用户名或密码".to_string());
    }
    let host = url.host_str().ok_or("URL 缺少主机名")?.trim_end_matches('.');
    let normalized = host.to_ascii_lowercase();
    if normalized == "localhost"
        || normalized.ends_with(".localhost")
        || normalized.ends_with(".local")
        || normalized.ends_with(".internal")
    {
        return Err("目标不能指向本机或私有网络".to_string());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if !is_public_ip(ip) {
            return Err("目标不能指向本机或私有网络".to_string());
        }
    }
    Ok(url)
}

pub async fn resolve_public_target(url: &Url) -> Result<SocketAddr, String> {
    let host = url.host_str().ok_or("URL 缺少主机名")?;
    let port = url.port_or_known_default().ok_or("URL 缺少端口")?;
    if let Ok(ip) = host.parse::<IpAddr>() {
        let address = SocketAddr::new(ip, port);
        if !is_public_ip(ip) {
            return Err("目标不能指向本机或私有网络".to_string());
        }
        return Ok(address);
    }

    let addresses = tokio::time::timeout(
        Duration::from_secs(10),
        lookup_host((host, port)),
    )
    .await
    .map_err(|_| "目标 DNS 解析超时".to_string())?
    .map_err(|error| format!("目标 DNS 解析失败: {error}"))?;

    addresses
        .filter(|address| is_public_ip(address.ip()))
        .min_by_key(|address| if address.is_ipv4() { 0 } else { 1 })
        .ok_or_else(|| "目标不能指向本机或私有网络".to_string())
}

pub async fn public_client(url: &Url, timeout: Duration) -> Result<Client, String> {
    let address = resolve_public_target(url).await?;
    let host = url.host_str().ok_or("URL 缺少主机名")?;
    let mut builder = Client::builder()
        .timeout(timeout)
        .redirect(Policy::none())
        .user_agent("reader-next/secure-fetch");
    if host.parse::<IpAddr>().is_err() {
        builder = builder.resolve(host, address);
    }
    builder.build().map_err(|error| error.to_string())
}

pub fn public_client_blocking(
    url: &Url,
    timeout: Duration,
) -> Result<reqwest::blocking::Client, String> {
    let host = url.host_str().ok_or("URL 缺少主机名")?;
    let port = url.port_or_known_default().ok_or("URL 缺少端口")?;
    let address = if let Ok(ip) = host.parse::<IpAddr>() {
        SocketAddr::new(ip, port)
    } else {
        (host, port)
            .to_socket_addrs()
            .map_err(|error| format!("目标 DNS 解析失败: {error}"))?
            .filter(|address| is_public_ip(address.ip()))
            .min_by_key(|address| if address.is_ipv4() { 0 } else { 1 })
            .ok_or_else(|| "目标不能指向本机或私有网络".to_string())?
    };
    if !is_public_ip(address.ip()) {
        return Err("目标不能指向本机或私有网络".to_string());
    }

    let mut builder = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .redirect(Policy::none())
        .user_agent("reader-next/secure-fetch");
    if host.parse::<IpAddr>().is_err() {
        builder = builder.resolve(host, address);
    }
    builder.build().map_err(|error| error.to_string())
}

pub async fn read_bytes_limited(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("上游响应超过大小限制".to_string());
    }

    let mut output = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("读取上游响应失败: {error}"))?
    {
        if output.len().saturating_add(chunk.len()) > limit {
            return Err("上游响应超过大小限制".to_string());
        }
        output.extend_from_slice(&chunk);
    }
    Ok(output)
}

pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    if ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_unspecified()
        || ip.is_multicast()
    {
        return false;
    }

    let value = u32::from(ip);
    let ranges = [
        (0x0000_0000u32, 0xff00_0000u32),
        (0x6440_0000, 0xffc0_0000),
        (0xc000_0000, 0xffffff00),
        (0xc000_0200, 0xffffff00),
        (0xc612_0000, 0xfffe_0000),
        (0xc633_6400, 0xffffff00),
        (0xcb00_7100, 0xffffff00),
        (0xe000_0000, 0xf000_0000),
        (0xf000_0000, 0xf000_0000),
    ];
    ranges
        .iter()
        .all(|(network, mask)| (value & mask) != *network)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
        return false;
    }
    if let Some(mapped) = ip.to_ipv4() {
        return is_public_ipv4(mapped);
    }

    let segments = ip.segments();
    let unique_local = segments[0] & 0xfe00 == 0xfc00;
    let link_local = segments[0] & 0xffc0 == 0xfe80;
    let documentation = segments[0] == 0x2001 && segments[1] == 0x0db8;
    let discard = segments[0] == 0x0100 && segments[1..5] == [0, 0, 0, 0];
    !(unique_local || link_local || documentation || discard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_private_and_non_http_targets() {
        for target in [
            "file:///tmp/secret",
            "http://127.0.0.1/admin",
            "http://10.0.0.1/admin",
            "http://[::1]/admin",
            "http://localhost/admin",
            "http://metadata.google.internal/computeMetadata/v1/",
        ] {
            assert!(parse_public_http_url(target).is_err(), "{target}");
        }
    }

    #[test]
    fn accepts_public_http_url() {
        assert!(parse_public_http_url("https://example.com/path").is_ok());
    }
}

pub fn read_bytes_limited_blocking(
    mut response: reqwest::blocking::Response,
    limit: usize,
) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err("上游响应超过大小限制".to_string());
    }
    let mut output = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut output)
        .map_err(|error| format!("读取上游响应失败: {error}"))?;
    if output.len() > limit {
        return Err("上游响应超过大小限制".to_string());
    }
    Ok(output)
}

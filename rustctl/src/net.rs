use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Address {
    pub(crate) interface: String,
    pub(crate) ip: Ipv4Addr,
}

fn interface_rank(name: &str) -> u8 {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("eth") || lower.starts_with("en") {
        0
    } else if lower.starts_with("wlan") || lower.starts_with("wl") {
        1
    } else if lower.starts_with("usb") {
        2
    } else {
        3
    }
}

fn is_virtual_interface(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("docker")
        || lower.starts_with("br-")
        || lower.starts_with("veth")
        || lower.starts_with("virbr")
}

pub(crate) fn rank_addresses(mut candidates: Vec<Address>) -> Vec<Address> {
    candidates.retain(|a| !a.ip.is_loopback() && !is_virtual_interface(&a.interface));
    candidates.sort_by(|a, b| {
        a.ip
            .cmp(&b.ip)
            .then_with(|| interface_rank(&a.interface).cmp(&interface_rank(&b.interface)))
            .then_with(|| a.interface.cmp(&b.interface))
    });
    candidates.dedup_by(|a, b| a.ip == b.ip);
    candidates.sort_by(|a, b| {
        interface_rank(&a.interface)
            .cmp(&interface_rank(&b.interface))
            .then_with(|| a.interface.cmp(&b.interface))
            .then_with(|| a.ip.cmp(&b.ip))
    });
    candidates
}

pub(crate) fn discover_addresses() -> Vec<Address> {
    let ifaces = if_addrs::get_if_addrs().unwrap_or_default();
    rank_ip_addresses(ifaces.into_iter().map(|iface| {
        let ip = iface.ip();
        (iface.name, ip)
    }))
}

pub(crate) fn rank_ip_addresses(
    candidates: impl IntoIterator<Item = (String, IpAddr)>,
) -> Vec<Address> {
    let candidates = candidates
        .into_iter()
        .filter_map(|(interface, ip)| match ip {
            IpAddr::V4(ip) => Some(Address { interface, ip }),
            std::net::IpAddr::V6(_) => None,
        })
        .collect();
    rank_addresses(candidates)
}

pub(crate) fn read_hostname_from(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

pub(crate) fn read_hostname() -> Option<String> {
    read_hostname_from(Path::new("/proc/sys/kernel/hostname"))
}

pub(crate) fn hostname_url(hostname: &str, port: u16) -> Option<String> {
    if hostname.is_empty() {
        return None;
    }
    let label = if hostname.contains('.') {
        hostname.to_owned()
    } else {
        format!("{hostname}.local")
    };
    Some(format!("http://{label}:{port}/"))
}

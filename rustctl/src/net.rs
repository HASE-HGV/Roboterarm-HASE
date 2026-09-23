use std::net::Ipv4Addr;

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
        interface_rank(&a.interface)
            .cmp(&interface_rank(&b.interface))
            .then_with(|| a.interface.cmp(&b.interface))
            .then_with(|| a.ip.cmp(&b.ip))
    });
    candidates.dedup_by(|a, b| a.ip == b.ip);
    candidates
}

pub(crate) fn discover_addresses() -> Vec<Address> {
    let ifaces = if_addrs::get_if_addrs().unwrap_or_default();
    let candidates = ifaces
        .into_iter()
        .filter_map(|iface| match iface.ip() {
            std::net::IpAddr::V4(ip) => Some(Address {
                interface: iface.name,
                ip,
            }),
            std::net::IpAddr::V6(_) => None,
        })
        .collect();
    rank_addresses(candidates)
}

pub(crate) fn read_hostname() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
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

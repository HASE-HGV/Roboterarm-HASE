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

/// Drops loopback and virtual interfaces, de-duplicates identical IPs, and
/// orders the rest wired-before-wireless-before-other so the most useful
/// address for someone plugging in an Ethernet cable is listed first.
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

/// Real system call: every IPv4 address on every network interface, ranked
/// for display. Not itself unit-tested (it depends on the machine it runs
/// on) - `rank_addresses` carries the logic and is tested directly.
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

/// Real system call: the device's hostname, trimmed, or `None` if it can't
/// be read or is empty.
pub(crate) fn read_hostname() -> Option<String> {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// Builds an `http://` URL from a hostname, appending `.local` to
/// single-label names (`raspberrypi` -> `raspberrypi.local`) since that is
/// what mDNS/Avahi actually advertises; already-qualified names
/// (`pi.lan`, `x.local`) are used as given.
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
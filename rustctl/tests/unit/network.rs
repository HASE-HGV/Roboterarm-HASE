use super::*;

#[test]
fn a_domain_label() {
    announce_domain("Network discovery and site binding");
}

#[test]
fn net_filters_loopback_and_virtual_interfaces() {
    let ranked = rank_addresses(vec![
        addr("lo", [127, 0, 0, 1]),
        addr("docker0", [172, 17, 0, 1]),
        addr("veth1234", [172, 18, 0, 1]),
        addr("br-abcdef", [172, 19, 0, 1]),
        addr("virbr0", [192, 168, 122, 1]),
        addr("eth0", [192, 168, 1, 50]),
    ]);
    assert_eq!(ranked, vec![addr("eth0", [192, 168, 1, 50])]);
}

#[test]
fn net_ranks_wired_before_wireless_before_usb_before_other() {
    let ranked = rank_addresses(vec![
        addr("wlan0", [192, 168, 1, 20]),
        addr("usb0", [192, 168, 2, 20]),
        addr("eth0", [192, 168, 1, 50]),
        addr("tun0", [10, 0, 0, 5]),
    ]);
    let order: Vec<&str> = ranked.iter().map(|a| a.interface.as_str()).collect();
    assert_eq!(order, vec!["eth0", "wlan0", "usb0", "tun0"]);
}

#[test]
fn net_dedups_duplicate_ip_addresses() {
    let ranked = rank_addresses(vec![
        addr("eth0", [192, 168, 1, 50]),
        addr("eth0:1", [192, 168, 1, 50]),
    ]);
    assert_eq!(ranked.len(), 1);
}

#[test]
fn net_orders_addresses_on_the_same_interface_by_ip() {
    let ranked = rank_addresses(vec![
        addr("eth0", [192, 168, 1, 20]),
        addr("eth0", [192, 168, 1, 10]),
    ]);
    assert_eq!(
        ranked,
        vec![
            addr("eth0", [192, 168, 1, 10]),
            addr("eth0", [192, 168, 1, 20]),
        ]
    );
}

#[test]
fn net_empty_input_yields_empty_output() {
    assert!(rank_addresses(vec![]).is_empty());
}

#[test]
fn net_ip_discovery_keeps_ipv4_and_skips_ipv6() {
    let ranked = rank_ip_addresses([
        ("wlan0".to_owned(), "192.168.1.20".parse().unwrap()),
        ("eth0".to_owned(), "192.168.1.10".parse().unwrap()),
        ("eth0".to_owned(), "fe80::1".parse().unwrap()),
    ]);
    assert_eq!(ranked, vec![
        addr("eth0", [192, 168, 1, 10]),
        addr("wlan0", [192, 168, 1, 20]),
    ]);
}

#[test]
fn net_discovered_addresses_exclude_loopback() {
    let addresses = discover_addresses();
    assert!(addresses.iter().all(|address| !address.ip.is_loopback()));
}

#[test]
fn net_hostname_file_is_trimmed_and_empty_content_is_ignored() {
    assert_eq!(read_hostname(), read_hostname_from(std::path::Path::new(
        "/proc/sys/kernel/hostname",
    )));

    let path = std::env::temp_dir().join(format!("rustctl-hostname-{}", std::process::id()));
    std::fs::write(&path, " robot-arm.local \n").unwrap();
    assert_eq!(read_hostname_from(&path).as_deref(), Some("robot-arm.local"));
    std::fs::write(&path, "  \n").unwrap();
    assert_eq!(read_hostname_from(&path), None);
    std::fs::remove_file(&path).unwrap();
    assert_eq!(read_hostname_from(&path), None);
}

#[test]
fn net_hostname_url_rejects_empty_hostname() {
    assert_eq!(hostname_url("", 8080), None);
}

#[test]
fn net_hostname_url_adds_local_domain_to_short_name() {
    assert_eq!(
        hostname_url("robot-arm", 8080).as_deref(),
        Some("http://robot-arm.local:8080/")
    );
}

#[test]
fn net_hostname_url_preserves_fully_qualified_hostname() {
    assert_eq!(
        hostname_url("robot-arm.example", 8080).as_deref(),
        Some("http://robot-arm.example:8080/")
    );
}

#[test]
fn site_bind_address_prefers_nonempty_site_setting() {
    let address = resolve_bind_address_with(|name| match name {
        "RUSTCTL_SITE_ADDR" => Some(" 127.0.0.1:9000 ".to_owned()),
        "RUSTCTL_API_ADDR" => Some("127.0.0.1:9001".to_owned()),
        _ => None,
    });
    assert_eq!(address, ("127.0.0.1:9000".to_owned(), false));
}

#[test]
fn site_bind_address_uses_legacy_setting_when_new_setting_is_blank() {
    let address = resolve_bind_address_with(|name| match name {
        "RUSTCTL_SITE_ADDR" => Some("  ".to_owned()),
        "RUSTCTL_API_ADDR" => Some(" 127.0.0.1:9001 ".to_owned()),
        _ => None,
    });
    assert_eq!(address, ("127.0.0.1:9001".to_owned(), true));
}

#[test]
fn site_bind_address_defaults_when_both_configured_values_are_blank() {
    let address = resolve_bind_address_with(|name| match name {
        "RUSTCTL_SITE_ADDR" | "RUSTCTL_API_ADDR" => Some("  ".to_owned()),
        _ => None,
    });
    assert_eq!(address, ("0.0.0.0:8080".to_owned(), false));
}

#[test]
fn site_bind_address_defaults_to_wildcard_and_default_port() {
    let address = resolve_bind_address_with(|_| None);
    assert_eq!(address, ("0.0.0.0:8080".to_owned(), false));
}

#[test]
fn site_banner_shows_loopback_warning_without_network_addresses() {
    let wildcard = "0.0.0.0:8080".parse().unwrap();
    let no_network = banner_lines(wildcard, vec![], Some("".to_owned()), false).join("\n");
    assert!(
        no_network.contains("loopback only")
            && no_network.contains("No network address found")
            && no_network.contains("Simulation build"),
        "{no_network}"
    );
}

#[test]
fn site_banner_lists_network_and_hostname_urls_for_wildcard_bind() {
    let wildcard = "0.0.0.0:8080".parse().unwrap();
    let network = banner_lines(
        wildcard,
        vec![addr("eth0", [192, 168, 1, 10])],
        Some("robot-arm".to_owned()),
        true,
    )
    .join("\n");
    assert!(
        network.contains("http://192.168.1.10:8080/  (eth0)")
            && network.contains("robot-arm.local:8080")
            && !network.contains("Simulation build"),
        "{network}"
    );
}

#[test]
fn site_banner_uses_loopback_when_hostname_is_absent() {
    let wildcard = "0.0.0.0:8080".parse().unwrap();
    let no_hostname = banner_lines(wildcard, vec![], None, true).join("\n");
    assert!(no_hostname.contains("loopback only"), "{no_hostname}");
}

#[test]
fn site_banner_displays_explicit_non_wildcard_bind_address() {
    let bound = banner_lines("127.0.0.1:1234".parse().unwrap(), vec![], None, true).join("\n");
    assert!(bound.contains("http://127.0.0.1:1234/"), "{bound}");
}

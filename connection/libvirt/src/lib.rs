/* Copyright 2024-2026 Marco Köpcke
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
pub use hypervisor::*;

use std::net::IpAddr;

mod connection;
mod hypervisor;
mod qemu_preferences;

static LOCALHOST_NAMES: &[&str] = &[
    "localhost",
    "localhost.localdomain",
    "localhost4",
    "localhost4.localdomain4",
    "localhost6",
    "localhost6.localdomain6",
    "ip6-localhost",
    "ip6-loopback",
];

/// Returns `true` if the given hostname or IP address refers to the local machine.
pub fn is_localhost(hostname: &str) -> bool {
    // Normalize: trailing root dot of a FQDN and brackets around IPv6 literals.
    let hostname = hostname.strip_suffix('.').unwrap_or(hostname);
    let hostname = hostname
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(hostname);

    let lower = hostname.to_ascii_lowercase();
    if LOCALHOST_NAMES.contains(&lower.as_str()) || lower.ends_with(".localhost") {
        return true;
    }

    match hostname.parse::<IpAddr>() {
        Ok(IpAddr::V4(addr)) => addr.is_loopback(),
        Ok(IpAddr::V6(addr)) => {
            addr.is_loopback() || addr.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback())
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_localhost;

    #[test]
    fn matches_localhost_hostnames() {
        assert!(is_localhost("localhost"));
        assert!(is_localhost("Localhost"));
        assert!(is_localhost("LOCALHOST"));
        assert!(is_localhost("localhost."));
        assert!(is_localhost("localhost.localdomain"));
        assert!(is_localhost("localhost4"));
        assert!(is_localhost("localhost4.localdomain4"));
        assert!(is_localhost("localhost6"));
        assert!(is_localhost("localhost6.localdomain6"));
        assert!(is_localhost("ip6-localhost"));
        assert!(is_localhost("ip6-loopback"));
        assert!(is_localhost("foo.localhost"));
        assert!(is_localhost("foo.bar.localhost"));
    }

    #[test]
    fn matches_loopback_ips() {
        assert!(is_localhost("127.0.0.1"));
        assert!(is_localhost("127.0.0.53"));
        assert!(is_localhost("127.255.255.255"));
        assert!(is_localhost("::1"));
        assert!(is_localhost("[::1]"));
        assert!(is_localhost("0:0:0:0:0:0:0:1"));
        assert!(is_localhost("::ffff:127.0.0.1"));
    }

    #[test]
    fn rejects_non_localhost() {
        assert!(!is_localhost(""));
        assert!(!is_localhost("example.com"));
        assert!(!is_localhost("localhost.example.com"));
        assert!(!is_localhost("notlocalhost"));
        assert!(!is_localhost("192.168.1.1"));
        assert!(!is_localhost("10.0.0.1"));
        assert!(!is_localhost("128.0.0.1"));
        assert!(!is_localhost("0.0.0.0"));
        assert!(!is_localhost("::"));
        assert!(!is_localhost("fe80::1"));
        assert!(!is_localhost("::ffff:192.168.1.1"));
    }
}

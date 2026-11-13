//! The IPv4 address and name servers of a saved connection: what the form accepts and the
//! `nmcli` arguments that apply it. NetworkManager does the rest.

use std::net::{IpAddr, Ipv4Addr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    Auto,
    Manual,
    LinkLocal,
    Disabled,
}

impl Method {
    pub const ALL: [Method; 4] = [
        Method::Auto,
        Method::Manual,
        Method::LinkLocal,
        Method::Disabled,
    ];

    /// The name NetworkManager gives the method.
    pub fn nm_name(self) -> &'static str {
        match self {
            Method::Auto => "auto",
            Method::Manual => "manual",
            Method::LinkLocal => "link-local",
            Method::Disabled => "disabled",
        }
    }

    pub fn from_nm(name: &str) -> Self {
        Method::ALL
            .into_iter()
            .find(|m| m.nm_name() == name)
            .unwrap_or(Method::Auto)
    }

    pub fn title(self) -> &'static str {
        match self {
            Method::Auto => "Automatic (DHCP)",
            Method::Manual => "Manual",
            Method::LinkLocal => "Link-local only",
            Method::Disabled => "Disabled",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Form {
    pub method: Method,
    pub address: String,
    /// A prefix length (`24`) or a netmask (`255.255.255.0`).
    pub prefix: String,
    pub gateway: String,
    /// Whether the name servers the network hands out are used as well.
    pub auto_dns: bool,
    /// Name servers separated by spaces or commas.
    pub dns: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    BadAddress,
    BadPrefix,
    BadGateway,
    BadDns,
}

/// An IPv4 address in dotted form.
pub fn valid_ipv4(text: &str) -> bool {
    text.trim().parse::<Ipv4Addr>().is_ok()
}

/// A prefix length from `24`, `/24` or a netmask such as `255.255.255.0` (whose ones must come
/// first).
pub fn parse_prefix(text: &str) -> Option<u8> {
    let text = text.trim().trim_start_matches('/');
    if let Ok(length) = text.parse::<u8>() {
        return (length <= 32).then_some(length);
    }
    let mask = u32::from(text.parse::<Ipv4Addr>().ok()?);
    let ones = mask.leading_ones();
    (mask.checked_shl(ones).unwrap_or(0) == 0).then_some(ones as u8)
}

/// The name servers in a field: words split on spaces, commas and semicolons, each an address.
pub fn parse_dns(text: &str) -> Option<Vec<IpAddr>> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == ';')
        .filter(|word| !word.is_empty())
        .map(|word| word.parse::<IpAddr>().ok())
        .collect()
}

/// Checks what a manual configuration needs and any name servers given.
pub fn check(form: &Form) -> Result<(), Problem> {
    if form.method == Method::Manual {
        if !valid_ipv4(&form.address) {
            return Err(Problem::BadAddress);
        }
        if parse_prefix(&form.prefix).is_none() {
            return Err(Problem::BadPrefix);
        }
        if !form.gateway.trim().is_empty() && !valid_ipv4(&form.gateway) {
            return Err(Problem::BadGateway);
        }
    }
    match parse_dns(&form.dns) {
        Some(_) => Ok(()),
        None => Err(Problem::BadDns),
    }
}

/// The arguments for `nmcli connection modify` that make the connection what the form says.
/// IPv4 name servers go to `ipv4.dns` and IPv6 ones to `ipv6.dns`.
pub fn modify_args(uuid: &str, form: &Form) -> Vec<String> {
    let mut args: Vec<String> = ["nmcli", "connection", "modify", "uuid", uuid]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut set = |key: &str, value: String| {
        args.push(key.to_owned());
        args.push(value);
    };
    set("ipv4.method", form.method.nm_name().to_owned());
    if form.method == Method::Manual {
        let prefix = parse_prefix(&form.prefix).unwrap_or(24);
        set(
            "ipv4.addresses",
            format!("{}/{}", form.address.trim(), prefix),
        );
        set("ipv4.gateway", form.gateway.trim().to_owned());
    } else {
        set("ipv4.addresses", String::new());
        set("ipv4.gateway", String::new());
    }
    let servers = parse_dns(&form.dns).unwrap_or_default();
    let (v4, v6): (Vec<_>, Vec<_>) = servers.iter().partition(|ip| ip.is_ipv4());
    let join = |list: &[&IpAddr]| {
        list.iter()
            .map(|ip| ip.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    };
    set("ipv4.dns", join(&v4));
    set("ipv6.dns", join(&v6));
    // The name servers the network hands out are dropped when the person gives their own and
    // does not ask for both.
    let ignore = matches!(form.method, Method::Auto | Method::Manual)
        && !form.auto_dns
        && !servers.is_empty();
    set(
        "ipv4.ignore-auto-dns",
        if ignore { "yes" } else { "no" }.to_owned(),
    );
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(method: Method) -> Form {
        Form {
            method,
            address: "192.168.1.20".into(),
            prefix: "24".into(),
            gateway: "192.168.1.1".into(),
            auto_dns: false,
            dns: "1.1.1.1, 8.8.8.8".into(),
        }
    }

    #[test]
    fn prefixes_come_as_a_length_or_a_mask() {
        assert_eq!(parse_prefix("24"), Some(24));
        assert_eq!(parse_prefix("/16"), Some(16));
        assert_eq!(parse_prefix("255.255.255.0"), Some(24));
        assert_eq!(parse_prefix("255.255.0.0"), Some(16));
        assert_eq!(parse_prefix("255.255.255.255"), Some(32));
        assert_eq!(parse_prefix("0.0.0.0"), Some(0));
        assert_eq!(parse_prefix("33"), None);
        assert_eq!(parse_prefix("255.0.255.0"), None);
        assert_eq!(parse_prefix("abc"), None);
        assert_eq!(parse_prefix(""), None);
    }

    #[test]
    fn name_servers_are_addresses() {
        assert_eq!(parse_dns("").unwrap().len(), 0);
        assert_eq!(parse_dns("1.1.1.1,8.8.8.8 ; 9.9.9.9").unwrap().len(), 3);
        assert_eq!(parse_dns("2606:4700:4700::1111").unwrap().len(), 1);
        assert!(parse_dns("1.1.1.1 dns.example").is_none());
    }

    #[test]
    fn a_manual_form_needs_an_address_and_a_prefix() {
        assert_eq!(check(&form(Method::Manual)), Ok(()));
        let mut bad = form(Method::Manual);
        bad.address = "192.168.1".into();
        assert_eq!(check(&bad), Err(Problem::BadAddress));
        let mut bad = form(Method::Manual);
        bad.prefix = "99".into();
        assert_eq!(check(&bad), Err(Problem::BadPrefix));
        let mut bad = form(Method::Manual);
        bad.gateway = "nope".into();
        assert_eq!(check(&bad), Err(Problem::BadGateway));
        let mut free = form(Method::Manual);
        free.gateway = String::new();
        assert_eq!(check(&free), Ok(()));
        // An automatic connection is not asked for an address.
        let mut auto = form(Method::Auto);
        auto.address = "garbage".into();
        assert_eq!(check(&auto), Ok(()));
        let mut dns = form(Method::Auto);
        dns.dns = "x".into();
        assert_eq!(check(&dns), Err(Problem::BadDns));
    }

    #[test]
    fn the_arguments_apply_the_form() {
        let args = modify_args("abc", &form(Method::Manual));
        let text = args.join(" ");
        assert!(text.starts_with("nmcli connection modify uuid abc "));
        assert!(text.contains("ipv4.method manual"));
        assert!(text.contains("ipv4.addresses 192.168.1.20/24"));
        assert!(text.contains("ipv4.gateway 192.168.1.1"));
        assert!(text.contains("ipv4.dns 1.1.1.1 8.8.8.8"));
        assert!(text.contains("ipv4.ignore-auto-dns yes"));
        // Each value is one argument: nothing goes through a shell.
        assert_eq!(
            args[args.iter().position(|a| a == "ipv4.dns").unwrap() + 1],
            "1.1.1.1 8.8.8.8"
        );
    }

    #[test]
    fn automatic_clears_the_address_and_keeps_the_network_servers_on_request() {
        let mut f = form(Method::Auto);
        f.auto_dns = true;
        let args = modify_args("abc", &f);
        let at = |key: &str| args[args.iter().position(|a| a == key).unwrap() + 1].clone();
        assert_eq!(at("ipv4.method"), "auto");
        assert_eq!(at("ipv4.addresses"), "");
        assert_eq!(at("ipv4.gateway"), "");
        assert_eq!(at("ipv4.ignore-auto-dns"), "no");
        f.dns = String::new();
        assert_eq!(at_of(&modify_args("abc", &f), "ipv4.dns"), "");
    }

    fn at_of(args: &[String], key: &str) -> String {
        args[args.iter().position(|a| a == key).unwrap() + 1].clone()
    }

    #[test]
    fn ipv6_servers_go_to_the_ipv6_setting() {
        let mut f = form(Method::Auto);
        f.dns = "1.1.1.1 2606:4700:4700::1111".into();
        let args = modify_args("abc", &f);
        assert_eq!(at_of(&args, "ipv4.dns"), "1.1.1.1");
        assert_eq!(at_of(&args, "ipv6.dns"), "2606:4700:4700::1111");
    }

    #[test]
    fn methods_round_trip_through_networkmanagers_names() {
        for m in Method::ALL {
            assert_eq!(Method::from_nm(m.nm_name()), m);
        }
        assert_eq!(Method::from_nm("shared"), Method::Auto);
    }
}

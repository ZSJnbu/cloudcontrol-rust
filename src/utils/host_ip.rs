use std::net::UdpSocket;

/// Get the host machine's IP address by opening a UDP socket to 8.8.8.8.
pub fn get_host_ip() -> String {
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            if socket.connect("8.8.8.8:80").is_ok() {
                if let Ok(addr) = socket.local_addr() {
                    return addr.ip().to_string();
                }
            }
            "127.0.0.1".to_string()
        }
        Err(_) => "127.0.0.1".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_host_ip_not_empty() {
        let ip = get_host_ip();
        assert!(!ip.is_empty(), "Host IP should not be empty");
    }

    #[test]
    fn test_get_host_ip_returns_valid_ip() {
        let ip = get_host_ip();
        let parts: Vec<&str> = ip.split('.').collect();
        assert_eq!(parts.len(), 4, "IP should have 4 octets: {}", ip);
        for part in parts {
            let num: u8 = part.parse().expect("Each octet should be a valid u8");
            assert!(num <= 255);
        }
    }
}

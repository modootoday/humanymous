//! Bounded owned .invalid DNS control frames shared by lab actors, no resolver.
pub fn question(packet: &[u8]) -> Result<String, &'static str> {
    if packet.len() < 17
        || packet.len() > 512
        || packet[2] & 0x80 != 0
        || packet[4..12] != [0, 1, 0, 0, 0, 0, 0, 0]
    {
        return Err("DNS query framing");
    }
    let mut offset = 12;
    let mut labels = Vec::new();
    loop {
        let length = usize::from(*packet.get(offset).ok_or("DNS label")?);
        offset += 1;
        if length == 0 {
            break;
        }
        if length > 63 {
            return Err("DNS compression/label refused");
        }
        let label = packet
            .get(offset..offset + length)
            .ok_or("DNS label bounds")?;
        if !label
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || *b == b'-')
        {
            return Err("DNS owned label");
        }
        labels.push(
            std::str::from_utf8(label)
                .map_err(|_| "DNS ASCII")?
                .to_ascii_lowercase(),
        );
        offset += length;
        if offset > 253 {
            return Err("DNS name bound");
        }
    }
    if packet.len() != offset + 4
        || !matches!(&packet[offset..offset + 2], [0, 1] | [0, 28])
        || packet[offset + 2..] != [0, 1]
    {
        return Err("DNS question bounds/type");
    }
    let value = labels.join(".");
    if !value.ends_with(".invalid") {
        return Err("only owned invalid names accepted");
    }
    Ok(value)
}
pub fn query(value: &str) -> Result<Vec<u8>, &'static str> {
    if value.len() > 240 || !value.ends_with(".invalid") {
        return Err("owned DNS name required");
    }
    let mut bytes = vec![0x48, 0x4d, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in value.split('.') {
        if label.is_empty()
            || label.len() > 63
            || !label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        {
            return Err("owned DNS label required");
        }
        bytes.push(label.len() as u8);
        bytes.extend_from_slice(label.as_bytes());
    }
    bytes.extend_from_slice(&[0, 0, 1, 0, 1]);
    Ok(bytes)
}
pub fn reply(packet: &[u8], rcode: u8) -> Result<Vec<u8>, &'static str> {
    if !(12..=512).contains(&packet.len()) || packet[2] & 0x80 != 0 || !matches!(rcode, 3 | 5) {
        return Err("DNS control response framing");
    }
    let mut reply = packet.to_vec();
    reply[2] = 0x81;
    reply[3] = rcode;
    Ok(reply)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_questions_reject_compression_and_unowned_names() {
        let packet = query("positive-control.invalid").unwrap();
        assert_eq!(question(&packet).unwrap(), "positive-control.invalid");
        assert_eq!(reply(&packet, 5).unwrap()[3], 5);
        assert!(query("example.com").is_err());
        let mut bad = packet.clone();
        bad[12] = 0xc0;
        assert!(question(&bad).is_err());
        let mut bad = packet;
        bad.push(0);
        assert!(question(&bad).is_err());
    }
}

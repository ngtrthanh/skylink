/// NMEA sentence parser for !AIVDM/!AIVDO
/// Handles multi-sentence messages and extracts 6-bit ASCII payload

pub struct NmeaCollector {
    /// Pending multi-sentence fragments: (frag_count, frag_id) -> accumulated payload
    pending: std::collections::HashMap<(u8, u8), (u8, String)>,
}

impl NmeaCollector {
    pub fn new() -> Self { Self { pending: std::collections::HashMap::new() } }

    /// Feed a raw NMEA line. Returns Some(payload_bits) when a complete message is ready.
    pub fn feed(&mut self, line: &str) -> Option<Vec<u8>> {
        let line = line.trim();
        if !line.starts_with('!') { return None; }

        // Verify checksum
        let (body, expected) = line[1..].split_once('*')?;
        let calc = body.bytes().fold(0u8, |a, b| a ^ b);
        let exp = u8::from_str_radix(expected.trim(), 16).ok()?;
        if calc != exp { return None; }

        let parts: Vec<&str> = body.split(',').collect();
        if parts.len() < 7 { return None; }

        // parts[0] = AIVDM or AIVDO
        let tag = parts[0];
        if tag != "AIVDM" && tag != "AIVDO" { return None; }

        let frag_count: u8 = parts[1].parse().ok()?;
        let frag_num: u8 = parts[2].parse().ok()?;
        let seq_id: u8 = parts[3].parse().unwrap_or(0);
        // parts[4] = channel (A/B)
        let payload = parts[5];

        if frag_count == 1 {
            return Some(decode_payload(payload));
        }

        // Multi-sentence
        let key = (frag_count, seq_id);
        let entry = self.pending.entry(key).or_insert((0, String::new()));
        entry.0 = frag_num;
        entry.1.push_str(payload);

        if frag_num == frag_count {
            let full = self.pending.remove(&key)?;
            return Some(decode_payload(&full.1));
        }
        None
    }
}

/// Decode 6-bit ASCII payload to bit vector (packed as bytes, MSB first)
fn decode_payload(payload: &str) -> Vec<u8> {
    let mut bits = Vec::with_capacity(payload.len() * 6 / 8 + 1);
    let mut accum: u32 = 0;
    let mut nbits = 0;

    for ch in payload.bytes() {
        let mut v = (ch - 48) as u32;
        if v > 40 { v -= 8; }
        accum = (accum << 6) | (v & 0x3F);
        nbits += 6;
        while nbits >= 8 {
            nbits -= 8;
            bits.push((accum >> nbits) as u8);
        }
    }
    // Remaining bits (if any) left-aligned
    if nbits > 0 {
        bits.push((accum << (8 - nbits)) as u8);
    }
    bits
}

/// Extract unsigned integer from bit payload
pub fn get_uint(bits: &[u8], start: usize, len: usize) -> u32 {
    let mut val: u32 = 0;
    for i in 0..len {
        let byte_idx = (start + i) / 8;
        let bit_idx = 7 - ((start + i) % 8);
        if byte_idx < bits.len() && (bits[byte_idx] >> bit_idx) & 1 == 1 {
            val |= 1 << (len - 1 - i);
        }
    }
    val
}

/// Extract signed integer from bit payload
pub fn get_int(bits: &[u8], start: usize, len: usize) -> i32 {
    let u = get_uint(bits, start, len);
    if u & (1 << (len - 1)) != 0 {
        (u as i32) - (1 << len)
    } else {
        u as i32
    }
}

/// Extract 6-bit ASCII string from bit payload
pub fn get_string(bits: &[u8], start: usize, len_bits: usize) -> String {
    let mut s = String::new();
    let nchars = len_bits / 6;
    for i in 0..nchars {
        let v = get_uint(bits, start + i * 6, 6) as u8;
        let ch = if v < 32 { v + 64 } else { v };
        if ch == b'@' { break; }
        s.push(ch as char);
    }
    s.trim_end().to_string()
}

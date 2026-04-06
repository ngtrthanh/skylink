/// Beast binary protocol parser
/// Frame format: 0x1a <type> <6-byte timestamp> <1-byte signal> <payload>
/// Type '1' = Mode AC (2 bytes), '2' = Mode S short (7 bytes), '3' = Mode S long (14 bytes)
/// 0x1a bytes in payload are escaped as 0x1a 0x1a

pub struct BeastFrame {
    pub msg_type: u8,
    pub timestamp: u64,
    pub signal: u8,
    pub payload: Vec<u8>,
}

/// Extract Beast frames from a byte buffer.
/// Returns (frames, bytes_consumed).
pub fn extract_frames(buf: &[u8]) -> (Vec<BeastFrame>, usize) {
    let mut frames = Vec::new();
    let mut pos = 0;

    while pos < buf.len() {
        // Find 0x1a start
        if buf[pos] != 0x1a {
            pos += 1;
            continue;
        }
        if pos + 1 >= buf.len() {
            break; // incomplete
        }

        let msg_type = buf[pos + 1];
        let payload_len = match msg_type {
            b'1' => 2,  // Mode AC
            b'2' => 7,  // Mode S short
            b'3' => 14, // Mode S long
            0x1a => { pos += 2; continue; } // escaped 0x1a
            _ => { pos += 2; continue; } // unknown type, skip
        };

        // Need: 0x1a + type + 6 timestamp + 1 signal + payload (all potentially escaped)
        let start = pos;
        pos += 2; // skip 0x1a + type

        // Read timestamp (6 bytes, with escape handling)
        let mut timestamp: u64 = 0;
        let mut ts_ok = true;
        for _ in 0..6 {
            if pos >= buf.len() { ts_ok = false; break; }
            let mut b = buf[pos]; pos += 1;
            if b == 0x1a {
                if pos >= buf.len() { ts_ok = false; break; }
                b = buf[pos]; pos += 1;
                if b != 0x1a { pos -= 1; ts_ok = false; break; } // not a double escape
            }
            timestamp = (timestamp << 8) | b as u64;
        }
        if !ts_ok { pos = start + 2; continue; }

        // Read signal (1 byte)
        if pos >= buf.len() { pos = start; break; }
        let mut signal = buf[pos]; pos += 1;
        if signal == 0x1a {
            if pos >= buf.len() { pos = start; break; }
            signal = buf[pos]; pos += 1;
        }

        // Read payload
        let mut payload = Vec::with_capacity(payload_len);
        let mut pay_ok = true;
        for _ in 0..payload_len {
            if pos >= buf.len() { pay_ok = false; break; }
            let mut b = buf[pos]; pos += 1;
            if b == 0x1a {
                if pos >= buf.len() { pay_ok = false; break; }
                b = buf[pos]; pos += 1;
                if b != 0x1a { pos -= 1; pay_ok = false; break; }
            }
            payload.push(b);
        }
        if !pay_ok { pos = start; break; } // incomplete frame, retry later

        frames.push(BeastFrame {
            msg_type,
            timestamp,
            signal,
            payload,
        });
    }

    (frames, pos)
}

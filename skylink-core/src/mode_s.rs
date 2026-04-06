/// Mode S / ADS-B decoder — from ICAO Annex 10 spec
/// No external dependencies. Pure byte manipulation.

/// Decoded Mode S message
pub struct Message {
    pub df: u8,           // Downlink Format (0-24)
    pub icao: u32,        // 24-bit ICAO address
    pub altitude: Option<i32>,
    pub squawk: Option<u16>,
    pub callsign: Option<String>,
    pub cpr_lat: Option<u32>,
    pub cpr_lon: Option<u32>,
    pub cpr_odd: Option<bool>,
    pub airborne: bool,
    pub ground_speed: Option<f64>,
    pub ground_track: Option<f64>,
    pub vert_rate: Option<i32>,
    pub alt_gnss: Option<i32>,
    pub category: Option<u8>,
}

/// Decode a Mode S frame (7 or 14 bytes, already un-escaped from Beast)
pub fn decode(msg: &[u8]) -> Option<Message> {
    if msg.len() < 7 { return None; }

    let df = msg[0] >> 3;

    match df {
        0 => decode_df0(msg),           // Short air-air surveillance
        4 => decode_df4(msg),           // Surveillance altitude reply
        5 => decode_df5(msg),           // Surveillance identity reply
        11 => decode_df11(msg),         // All-call reply
        16 => decode_df16(msg),         // Long air-air surveillance
        17 | 18 => decode_df17(msg),    // Extended squitter (ADS-B)
        20 => decode_df20(msg),         // Comm-B altitude reply
        21 => decode_df21(msg),         // Comm-B identity reply
        _ => None,
    }
}

fn decode_df0(msg: &[u8]) -> Option<Message> {
    let alt = decode_ac13(&msg[2..4]);
    let icao = crc_residual(msg, 7);
    Some(Message {
        df: 0, icao, altitude: alt, airborne: true,
        ..Message::empty()
    })
}

fn decode_df4(msg: &[u8]) -> Option<Message> {
    let alt = decode_ac13(&msg[2..4]);
    let icao = crc_residual(msg, 7);
    Some(Message {
        df: 4, icao, altitude: alt, airborne: true,
        ..Message::empty()
    })
}

fn decode_df5(msg: &[u8]) -> Option<Message> {
    let squawk = decode_id13(&msg[2..4]);
    let icao = crc_residual(msg, 7);
    Some(Message {
        df: 5, icao, squawk: Some(squawk), airborne: false,
        ..Message::empty()
    })
}

fn decode_df11(msg: &[u8]) -> Option<Message> {
    let icao = ((msg[1] as u32) << 16) | ((msg[2] as u32) << 8) | (msg[3] as u32);
    Some(Message {
        df: 11, icao, ..Message::empty()
    })
}

fn decode_df16(msg: &[u8]) -> Option<Message> {
    if msg.len() < 14 { return None; }
    let alt = decode_ac13(&msg[2..4]);
    let icao = crc_residual(msg, 14);
    Some(Message {
        df: 16, icao, altitude: alt, airborne: true,
        ..Message::empty()
    })
}

fn decode_df17(msg: &[u8]) -> Option<Message> {
    if msg.len() < 14 { return None; }

    let icao = ((msg[1] as u32) << 16) | ((msg[2] as u32) << 8) | (msg[3] as u32);
    let tc = msg[4] >> 3; // Type Code (bits 33-37)
    let me = &msg[4..11]; // ME field (56 bits)

    let mut m = Message { df: msg[0] >> 3, icao, ..Message::empty() };

    match tc {
        1..=4 => decode_identification(me, &mut m),
        5..=8 => decode_surface_position(me, &mut m),
        9..=18 => decode_airborne_position_baro(me, &mut m),
        19 => decode_airborne_velocity(me, &mut m),
        20..=22 => decode_airborne_position_gnss(me, &mut m),
        _ => {}
    }

    Some(m)
}

fn decode_df20(msg: &[u8]) -> Option<Message> {
    if msg.len() < 14 { return None; }
    let alt = decode_ac13(&msg[2..4]);
    let icao = crc_residual(msg, 14);
    Some(Message {
        df: 20, icao, altitude: alt, airborne: true,
        ..Message::empty()
    })
}

fn decode_df21(msg: &[u8]) -> Option<Message> {
    if msg.len() < 14 { return None; }
    let squawk = decode_id13(&msg[2..4]);
    let icao = crc_residual(msg, 14);
    Some(Message {
        df: 21, icao, squawk: Some(squawk),
        ..Message::empty()
    })
}

// --- ADS-B ME field decoders ---

const AIS_CHARSET: &[u8] = b"?ABCDEFGHIJKLMNOPQRSTUVWXYZ????? ???????????????0123456789??????";

fn decode_identification(me: &[u8], m: &mut Message) {
    m.category = Some(me[0] & 0x07);
    let mut cs = String::with_capacity(8);
    let bits = u64::from_be_bytes([0, me[0], me[1], me[2], me[3], me[4], me[5], me[6]]);
    for i in 0..8 {
        let idx = ((bits >> (42 - i * 6)) & 0x3F) as usize;
        let ch = AIS_CHARSET.get(idx).copied().unwrap_or(b' ');
        if ch != b' ' || !cs.is_empty() {
            cs.push(ch as char);
        }
    }
    let cs = cs.trim_end().to_string();
    if !cs.is_empty() {
        m.callsign = Some(cs);
    }
}

fn decode_airborne_position_baro(me: &[u8], m: &mut Message) {
    m.airborne = true;
    // Altitude (AC12 field, bits 41-52 of ME)
    let ac12 = (((me[1] as u16) << 4) | ((me[2] as u16) >> 4)) & 0xFFF;
    m.altitude = decode_ac12(ac12);

    // CPR
    let odd = (me[2] >> 2) & 1;
    let lat_cpr = (((me[2] as u32 & 0x03) << 15) | ((me[3] as u32) << 7) | ((me[4] as u32) >> 1)) & 0x1FFFF;
    let lon_cpr = (((me[4] as u32 & 0x01) << 16) | ((me[5] as u32) << 8) | (me[6] as u32)) & 0x1FFFF;

    m.cpr_lat = Some(lat_cpr);
    m.cpr_lon = Some(lon_cpr);
    m.cpr_odd = Some(odd == 1);
}

fn decode_airborne_position_gnss(me: &[u8], m: &mut Message) {
    decode_airborne_position_baro(me, m);
    // For GNSS, altitude interpretation differs but CPR is same
    if let Some(alt) = m.altitude {
        m.alt_gnss = Some(alt);
        m.altitude = None; // GNSS alt goes to alt_gnss
    }
}

fn decode_surface_position(me: &[u8], m: &mut Message) {
    m.airborne = false;
    // Ground speed
    let movement = ((me[0] as u16 & 0x07) << 4) | ((me[1] as u16) >> 4);
    m.ground_speed = decode_movement(movement);

    let track_valid = (me[1] >> 3) & 1;
    if track_valid == 1 {
        let track_raw = ((me[1] as u16 & 0x07) << 4) | ((me[2] as u16) >> 4);
        m.ground_track = Some(track_raw as f64 * 360.0 / 128.0);
    }

    // CPR
    let odd = (me[2] >> 2) & 1;
    let lat_cpr = (((me[2] as u32 & 0x03) << 15) | ((me[3] as u32) << 7) | ((me[4] as u32) >> 1)) & 0x1FFFF;
    let lon_cpr = (((me[4] as u32 & 0x01) << 16) | ((me[5] as u32) << 8) | (me[6] as u32)) & 0x1FFFF;
    m.cpr_lat = Some(lat_cpr);
    m.cpr_lon = Some(lon_cpr);
    m.cpr_odd = Some(odd == 1);
}

fn decode_airborne_velocity(me: &[u8], m: &mut Message) {
    let subtype = me[0] & 0x07;
    match subtype {
        1 | 2 => {
            // Ground speed (subtype 1=subsonic, 2=supersonic)
            let ew_sign = (me[1] >> 2) & 1;
            let ew_vel = (((me[1] as i32 & 0x03) << 8) | me[2] as i32) - 1;
            let ns_sign = (me[3] >> 7) & 1;
            let ns_vel = (((me[3] as i32 & 0x7F) << 3) | (me[4] as i32 >> 5)) - 1;

            if ew_vel >= 0 && ns_vel >= 0 {
                let mult = if subtype == 2 { 4 } else { 1 };
                let vx = if ew_sign == 1 { -ew_vel * mult } else { ew_vel * mult };
                let vy = if ns_sign == 1 { -ns_vel * mult } else { ns_vel * mult };
                m.ground_speed = Some(((vx * vx + vy * vy) as f64).sqrt());
                m.ground_track = Some(((vx as f64).atan2(vy as f64).to_degrees() + 360.0) % 360.0);
            }

            // Vertical rate
            let vr_sign = (me[4] >> 3) & 1;
            let vr = (((me[4] as i32 & 0x07) << 6) | (me[5] as i32 >> 2)) - 1;
            if vr >= 0 {
                m.vert_rate = Some(if vr_sign == 1 { -vr * 64 } else { vr * 64 });
            }
        }
        3 | 4 => {
            // Airspeed + heading (not ground speed/track)
            // Skip for now — less common
        }
        _ => {}
    }
}

// --- Altitude decoders ---

fn decode_ac12(ac12: u16) -> Option<i32> {
    if ac12 == 0 { return None; }
    let q_bit = (ac12 >> 4) & 1;
    if q_bit == 1 {
        let n = ((ac12 & 0xFE0) >> 1) | (ac12 & 0x00F);
        Some(n as i32 * 25 - 1000)
    } else {
        // Gillham code — rare, skip for now
        None
    }
}

fn decode_ac13(bytes: &[u8]) -> Option<i32> {
    let ac13 = (((bytes[0] as u16) << 8) | bytes[1] as u16) & 0x1FFF;
    if ac13 == 0 { return None; }
    let m_bit = (ac13 >> 6) & 1;
    let q_bit = (ac13 >> 4) & 1;
    if m_bit == 0 && q_bit == 1 {
        let n = ((ac13 & 0x1F80) >> 2) | ((ac13 & 0x0020) >> 1) | (ac13 & 0x000F);
        Some(n as i32 * 25 - 1000)
    } else {
        None
    }
}

fn decode_id13(bytes: &[u8]) -> u16 {
    let id13 = (((bytes[0] as u16) << 8) | bytes[1] as u16) & 0x1FFF;
    // Decode Gillham-encoded squawk
    let a = ((id13 >> 10) & 0x07) as u16;
    let b = ((id13 >> 4) & 0x07) as u16;
    let c = ((id13 >> 1) & 0x07) as u16;
    let d = ((id13 & 0x01) | ((id13 >> 12) & 0x02) | ((id13 >> 7) & 0x04)) as u16;
    a * 1000 + b * 100 + c * 10 + d
}

fn decode_movement(movement: u16) -> Option<f64> {
    if movement == 0 { return None; }
    if movement == 1 { return Some(0.0); }
    let gs = match movement {
        2..=8 => (movement as f64 - 1.0) * 0.125,
        9..=12 => 1.0 + (movement as f64 - 9.0) * 0.25,
        13..=38 => 2.0 + (movement as f64 - 13.0) * 0.5,
        39..=93 => 15.0 + (movement as f64 - 39.0),
        94..=108 => 70.0 + (movement as f64 - 94.0) * 2.0,
        109..=123 => 100.0 + (movement as f64 - 109.0) * 5.0,
        124 => 175.0,
        _ => return None,
    };
    Some(gs)
}

// --- CRC ---

fn crc_residual(msg: &[u8], len: usize) -> u32 {
    let mut crc: u32 = 0;
    for i in 0..(len * 8) {
        let byte_idx = i / 8;
        let bit_idx = 7 - (i % 8);
        let bit = if byte_idx < msg.len() { (msg[byte_idx] >> bit_idx) & 1 } else { 0 };

        if (crc & 0x800000) != 0 {
            crc = ((crc << 1) | bit as u32) ^ 0xFFF409;
        } else {
            crc = (crc << 1) | bit as u32;
        }
    }
    crc & 0xFFFFFF
}

impl Message {
    fn empty() -> Self {
        Self {
            df: 0, icao: 0, altitude: None, squawk: None, callsign: None,
            cpr_lat: None, cpr_lon: None, cpr_odd: None, airborne: true,
            ground_speed: None, ground_track: None, vert_rate: None,
            alt_gnss: None, category: None,
        }
    }
}

/// AIS message decoder — types 1-5, 18-19, 21, 24
use super::nmea::{get_uint, get_int, get_string};
use super::vessel::VesselUpdate;

pub fn decode(bits: &[u8]) -> Option<VesselUpdate> {
    if bits.is_empty() { return None; }
    let msg_type = get_uint(bits, 0, 6) as u8;
    let mmsi = get_uint(bits, 8, 30);
    if mmsi == 0 { return None; }

    match msg_type {
        1 | 2 | 3 => decode_pos_a(bits, mmsi),
        5 => decode_static_a(bits, mmsi),
        18 => decode_pos_b(bits, mmsi),
        19 => decode_pos_b_ext(bits, mmsi),
        21 => decode_aton(bits, mmsi),
        24 => decode_static_b(bits, mmsi),
        _ => None,
    }
}

fn decode_pos_a(b: &[u8], mmsi: u32) -> Option<VesselUpdate> {
    if b.len() * 8 < 149 { return None; }
    let status = get_uint(b, 38, 4) as u8;
    let rot_raw = get_int(b, 42, 8);
    let sog = get_uint(b, 46, 10);
    let lon = get_int(b, 61, 28);
    let lat = get_int(b, 89, 27);
    let cog = get_uint(b, 116, 12);
    let hdg = get_uint(b, 128, 9);

    Some(VesselUpdate {
        mmsi,
        msg_type: get_uint(b, 0, 6) as u8,
        lat: if lat == 0x3412140 { None } else { Some(lat as f32 / 600000.0) },
        lon: if lon == 0x6791AC0 { None } else { Some(lon as f32 / 600000.0) },
        speed: if sog == 1023 { None } else { Some(sog as f32 / 10.0) },
        cog: if cog == 3600 { None } else { Some(cog as f32 / 10.0) },
        heading: if hdg == 511 { None } else { Some(hdg as u16) },
        status: Some(status),
        turn: if rot_raw == -128 { None } else { Some(rot_raw as i16) },
        shipclass: 1, // Class A
        ..Default::default()
    })
}

fn decode_static_a(b: &[u8], mmsi: u32) -> Option<VesselUpdate> {
    if b.len() * 8 < 424 { return None; }
    Some(VesselUpdate {
        mmsi,
        msg_type: 5,
        imo: { let v = get_uint(b, 40, 30); if v == 0 { None } else { Some(v) } },
        callsign: Some(get_string(b, 70, 42)),
        shipname: Some(get_string(b, 112, 120)),
        shiptype: Some(get_uint(b, 232, 8) as u8),
        to_bow: Some(get_uint(b, 240, 9) as u16),
        to_stern: Some(get_uint(b, 249, 9) as u16),
        to_port: Some(get_uint(b, 258, 6) as u16),
        to_starboard: Some(get_uint(b, 264, 6) as u16),
        draught: { let v = get_uint(b, 294, 8); if v == 0 { None } else { Some(v as f32 / 10.0) } },
        eta_month: Some(get_uint(b, 274, 4) as u8),
        eta_day: Some(get_uint(b, 278, 5) as u8),
        eta_hour: Some(get_uint(b, 283, 5) as u8),
        eta_minute: Some(get_uint(b, 288, 6) as u8),
        destination: Some(get_string(b, 302, 120)),
        shipclass: 1,
        ..Default::default()
    })
}

fn decode_pos_b(b: &[u8], mmsi: u32) -> Option<VesselUpdate> {
    if b.len() * 8 < 168 { return None; }
    let sog = get_uint(b, 46, 10);
    let lon = get_int(b, 57, 28);
    let lat = get_int(b, 85, 27);
    let cog = get_uint(b, 112, 12);
    let hdg = get_uint(b, 124, 9);

    Some(VesselUpdate {
        mmsi,
        msg_type: 18,
        lat: if lat == 0x3412140 { None } else { Some(lat as f32 / 600000.0) },
        lon: if lon == 0x6791AC0 { None } else { Some(lon as f32 / 600000.0) },
        speed: if sog == 1023 { None } else { Some(sog as f32 / 10.0) },
        cog: if cog == 3600 { None } else { Some(cog as f32 / 10.0) },
        heading: if hdg == 511 { None } else { Some(hdg as u16) },
        shipclass: 2, // Class B
        ..Default::default()
    })
}

fn decode_pos_b_ext(b: &[u8], mmsi: u32) -> Option<VesselUpdate> {
    if b.len() * 8 < 312 { return None; }
    let sog = get_uint(b, 46, 10);
    let lon = get_int(b, 57, 28);
    let lat = get_int(b, 85, 27);
    let cog = get_uint(b, 112, 12);
    let hdg = get_uint(b, 124, 9);

    Some(VesselUpdate {
        mmsi,
        msg_type: 19,
        lat: if lat == 0x3412140 { None } else { Some(lat as f32 / 600000.0) },
        lon: if lon == 0x6791AC0 { None } else { Some(lon as f32 / 600000.0) },
        speed: if sog == 1023 { None } else { Some(sog as f32 / 10.0) },
        cog: if cog == 3600 { None } else { Some(cog as f32 / 10.0) },
        heading: if hdg == 511 { None } else { Some(hdg as u16) },
        shipname: Some(get_string(b, 143, 120)),
        shiptype: Some(get_uint(b, 263, 8) as u8),
        to_bow: Some(get_uint(b, 271, 9) as u16),
        to_stern: Some(get_uint(b, 280, 9) as u16),
        to_port: Some(get_uint(b, 289, 6) as u16),
        to_starboard: Some(get_uint(b, 295, 6) as u16),
        shipclass: 2,
        ..Default::default()
    })
}

fn decode_aton(b: &[u8], mmsi: u32) -> Option<VesselUpdate> {
    if b.len() * 8 < 272 { return None; }
    let lon = get_int(b, 164, 28);
    let lat = get_int(b, 192, 27);

    Some(VesselUpdate {
        mmsi,
        msg_type: 21,
        lat: if lat == 0x3412140 { None } else { Some(lat as f32 / 600000.0) },
        lon: if lon == 0x6791AC0 { None } else { Some(lon as f32 / 600000.0) },
        shipname: Some(get_string(b, 40, 120)),
        shiptype: Some(get_uint(b, 38, 5) as u8), // ATON type in type field
        shipclass: 5, // ATON
        ..Default::default()
    })
}

fn decode_static_b(b: &[u8], mmsi: u32) -> Option<VesselUpdate> {
    let part = get_uint(b, 38, 2);
    match part {
        0 => {
            if b.len() * 8 < 160 { return None; }
            Some(VesselUpdate {
                mmsi, msg_type: 24,
                shipname: Some(get_string(b, 40, 120)),
                shipclass: 2,
                ..Default::default()
            })
        }
        1 => {
            if b.len() * 8 < 168 { return None; }
            Some(VesselUpdate {
                mmsi, msg_type: 24,
                shiptype: Some(get_uint(b, 40, 8) as u8),
                callsign: Some(get_string(b, 90, 42)),
                to_bow: Some(get_uint(b, 132, 9) as u16),
                to_stern: Some(get_uint(b, 141, 9) as u16),
                to_port: Some(get_uint(b, 150, 6) as u16),
                to_starboard: Some(get_uint(b, 156, 6) as u16),
                shipclass: 2,
                ..Default::default()
            })
        }
        _ => None,
    }
}

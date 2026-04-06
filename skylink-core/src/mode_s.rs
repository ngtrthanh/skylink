/// Mode S / ADS-B decoder — from ICAO Annex 10 spec
/// No external dependencies. Pure byte manipulation.

/// Decoded Mode S message
pub struct Message {
    pub df: u8,
    pub icao: u32,
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
    // New fields
    pub ias: Option<u16>,
    pub tas: Option<u16>,
    pub mach: Option<f64>,
    pub mag_heading: Option<f64>,
    pub true_heading: Option<f64>,
    pub roll: Option<f64>,
    pub track_rate: Option<f64>,
    pub geom_rate: Option<i32>,
    pub nav_altitude_mcp: Option<u32>,
    pub nav_altitude_fms: Option<u32>,
    pub nav_qnh: Option<f64>,
    pub nav_heading: Option<f64>,
    pub nav_modes: Option<u8>,
    pub nic: Option<u8>,
    pub nac_p: Option<u8>,
    pub nac_v: Option<u8>,
    pub sil: Option<u8>,
    pub sil_type: Option<u8>,
    pub gva: Option<u8>,
    pub sda: Option<u8>,
    pub nic_baro: Option<u8>,
    pub adsb_version: Option<u8>,
    pub emergency: Option<u8>,
    pub addr_type: u8, // 0=adsb_icao, 7=mode_s, etc
}

pub fn decode(msg: &[u8]) -> Option<Message> {
    if msg.len() < 7 { return None; }
    let df = msg[0] >> 3;
    match df {
        0 => decode_df0(msg),
        4 => decode_df4(msg),
        5 => decode_df5(msg),
        11 => decode_df11(msg),
        16 => decode_df16(msg),
        17 | 18 => decode_df17(msg),
        20 => decode_df20(msg),
        21 => decode_df21(msg),
        _ => None,
    }
}

fn decode_df0(msg: &[u8]) -> Option<Message> {
    Some(Message { df: 0, icao: crc_residual(msg, 7), altitude: decode_ac13(&msg[2..4]), airborne: true, addr_type: 7, ..Message::empty() })
}
fn decode_df4(msg: &[u8]) -> Option<Message> {
    Some(Message { df: 4, icao: crc_residual(msg, 7), altitude: decode_ac13(&msg[2..4]), airborne: true, addr_type: 7, ..Message::empty() })
}
fn decode_df5(msg: &[u8]) -> Option<Message> {
    Some(Message { df: 5, icao: crc_residual(msg, 7), squawk: Some(decode_id13(&msg[2..4])), addr_type: 7, ..Message::empty() })
}
fn decode_df11(msg: &[u8]) -> Option<Message> {
    let icao = ((msg[1] as u32) << 16) | ((msg[2] as u32) << 8) | (msg[3] as u32);
    Some(Message { df: 11, icao, addr_type: 7, ..Message::empty() })
}
fn decode_df16(msg: &[u8]) -> Option<Message> {
    if msg.len() < 14 { return None; }
    Some(Message { df: 16, icao: crc_residual(msg, 14), altitude: decode_ac13(&msg[2..4]), airborne: true, addr_type: 7, ..Message::empty() })
}
fn decode_df20(msg: &[u8]) -> Option<Message> {
    if msg.len() < 14 { return None; }
    let mut m = Message { df: 20, icao: crc_residual(msg, 14), altitude: decode_ac13(&msg[2..4]), airborne: true, addr_type: 7, ..Message::empty() };
    decode_bds(&msg[4..11], &mut m);
    Some(m)
}
fn decode_df21(msg: &[u8]) -> Option<Message> {
    if msg.len() < 14 { return None; }
    let mut m = Message { df: 21, icao: crc_residual(msg, 14), squawk: Some(decode_id13(&msg[2..4])), addr_type: 7, ..Message::empty() };
    decode_bds(&msg[4..11], &mut m);
    Some(m)
}

fn decode_df17(msg: &[u8]) -> Option<Message> {
    if msg.len() < 14 { return None; }
    let icao = ((msg[1] as u32) << 16) | ((msg[2] as u32) << 8) | (msg[3] as u32);
    let tc = msg[4] >> 3;
    let me = &msg[4..11];
    let addr_type = if msg[0] >> 3 == 18 { 8 } else { 0 }; // DF18 = adsb_other
    let mut m = Message { df: msg[0] >> 3, icao, addr_type, ..Message::empty() };

    match tc {
        1..=4 => decode_identification(me, &mut m),
        5..=8 => decode_surface_position(me, &mut m),
        9..=18 => decode_airborne_position_baro(me, &mut m),
        19 => decode_airborne_velocity(me, &mut m),
        20..=22 => decode_airborne_position_gnss(me, &mut m),
        28 => decode_aircraft_status(me, &mut m),
        29 => decode_target_state(me, &mut m),
        31 => decode_operational_status(me, &mut m),
        _ => {}
    }
    Some(m)
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
        if ch != b' ' || !cs.is_empty() { cs.push(ch as char); }
    }
    let cs = cs.trim_end().to_string();
    if !cs.is_empty() { m.callsign = Some(cs); }
}

fn decode_airborne_position_baro(me: &[u8], m: &mut Message) {
    m.airborne = true;
    let ac12 = (((me[1] as u16) << 4) | ((me[2] as u16) >> 4)) & 0xFFF;
    m.altitude = decode_ac12(ac12);
    let odd = (me[2] >> 2) & 1;
    m.cpr_lat = Some((((me[2] as u32 & 0x03) << 15) | ((me[3] as u32) << 7) | ((me[4] as u32) >> 1)) & 0x1FFFF);
    m.cpr_lon = Some((((me[4] as u32 & 0x01) << 16) | ((me[5] as u32) << 8) | (me[6] as u32)) & 0x1FFFF);
    m.cpr_odd = Some(odd == 1);
    // NIC from type code
    let tc = me[0] >> 3;
    m.nic = Some(match tc { 9 => 11, 10 => 10, 11 => 9, 12 => 8, 13 => 7, 14 => 6, 15 => 5, 16 => 4, 17 => 3, 18 => 2, _ => 0 });
}

fn decode_airborne_position_gnss(me: &[u8], m: &mut Message) {
    decode_airborne_position_baro(me, m);
    if let Some(alt) = m.altitude { m.alt_gnss = Some(alt); m.altitude = None; }
}

fn decode_surface_position(me: &[u8], m: &mut Message) {
    m.airborne = false;
    let movement = ((me[0] as u16 & 0x07) << 4) | ((me[1] as u16) >> 4);
    m.ground_speed = decode_movement(movement);
    if (me[1] >> 3) & 1 == 1 {
        let track_raw = ((me[1] as u16 & 0x07) << 4) | ((me[2] as u16) >> 4);
        m.ground_track = Some(track_raw as f64 * 360.0 / 128.0);
    }
    let odd = (me[2] >> 2) & 1;
    m.cpr_lat = Some((((me[2] as u32 & 0x03) << 15) | ((me[3] as u32) << 7) | ((me[4] as u32) >> 1)) & 0x1FFFF);
    m.cpr_lon = Some((((me[4] as u32 & 0x01) << 16) | ((me[5] as u32) << 8) | (me[6] as u32)) & 0x1FFFF);
    m.cpr_odd = Some(odd == 1);
}

fn decode_airborne_velocity(me: &[u8], m: &mut Message) {
    let subtype = me[0] & 0x07;
    match subtype {
        1 | 2 => {
            let mult = if subtype == 2 { 4 } else { 1 };
            let ew_sign = (me[1] >> 2) & 1;
            let ew_vel = (((me[1] as i32 & 0x03) << 8) | me[2] as i32) - 1;
            let ns_sign = (me[3] >> 7) & 1;
            let ns_vel = (((me[3] as i32 & 0x7F) << 3) | (me[4] as i32 >> 5)) - 1;
            if ew_vel >= 0 && ns_vel >= 0 {
                let vx = if ew_sign == 1 { -ew_vel * mult } else { ew_vel * mult };
                let vy = if ns_sign == 1 { -ns_vel * mult } else { ns_vel * mult };
                m.ground_speed = Some(((vx * vx + vy * vy) as f64).sqrt());
                m.ground_track = Some(((vx as f64).atan2(vy as f64).to_degrees() + 360.0) % 360.0);
            }
            // Vertical rate
            let vr_src = (me[4] >> 4) & 1; // 0=baro, 1=gnss
            let vr_sign = (me[4] >> 3) & 1;
            let vr = (((me[4] as i32 & 0x07) << 6) | (me[5] as i32 >> 2)) - 1;
            if vr >= 0 {
                let rate = if vr_sign == 1 { -vr * 64 } else { vr * 64 };
                if vr_src == 0 { m.vert_rate = Some(rate); } else { m.geom_rate = Some(rate); }
            }
            // GNSS/baro diff
            let diff_sign = (me[5] >> 1) & 1;
            let diff = (((me[5] as i32 & 0x01) << 6) | (me[6] as i32 >> 1)) - 1;
            if diff >= 0 { let _ = if diff_sign == 1 { -diff * 25 } else { diff * 25 }; }
            // NACv from subtype 1/2
            m.nac_v = Some(((me[1] >> 3) & 0x07) as u8);
        }
        3 | 4 => {
            // Airspeed + heading
            let mult = if subtype == 4 { 4 } else { 1 };
            let hdg_avail = (me[1] >> 2) & 1;
            if hdg_avail == 1 {
                let hdg_raw = ((me[1] as u16 & 0x03) << 8) | me[2] as u16;
                m.mag_heading = Some(hdg_raw as f64 * 360.0 / 1024.0);
            }
            let as_type = (me[3] >> 7) & 1; // 0=IAS, 1=TAS
            let as_val = (((me[3] as u16 & 0x7F) << 3) | (me[4] as u16 >> 5)) - 1;
            if as_val < 0x3FE {
                let speed = as_val * mult as u16;
                if as_type == 0 { m.ias = Some(speed); } else { m.tas = Some(speed); }
            }
            // Vertical rate same as subtype 1/2
            let vr_sign = (me[4] >> 3) & 1;
            let vr = (((me[4] as i32 & 0x07) << 6) | (me[5] as i32 >> 2)) - 1;
            if vr >= 0 {
                let rate = if vr_sign == 1 { -vr * 64 } else { vr * 64 };
                m.vert_rate = Some(rate);
            }
        }
        _ => {}
    }
}

/// TC=28: Aircraft Status (emergency/squawk)
fn decode_aircraft_status(me: &[u8], m: &mut Message) {
    let subtype = me[0] & 0x07;
    if subtype == 1 {
        m.emergency = Some((me[1] >> 5) & 0x07);
        let sq = ((me[1] as u16 & 0x1F) << 8) | me[2] as u16;
        m.squawk = Some(decode_id13_from_raw(sq));
    }
}

/// TC=29: Target State and Status
fn decode_target_state(me: &[u8], m: &mut Message) {
    let subtype = me[0] & 0x07;
    if subtype == 1 {
        // Version 2 target state
        let sil = (me[1] >> 6) & 0x03;
        m.sil = Some(sil);
        let sil_sup = (me[5] >> 4) & 1;
        m.sil_type = Some(sil_sup);
        // MCP altitude
        let alt_raw = ((me[1] as u32 & 0x3F) << 5) | (me[2] as u32 >> 3);
        if alt_raw > 0 { m.nav_altitude_mcp = Some((alt_raw - 1) * 32); }
        // Baro setting (QNH)
        let baro_raw = ((me[2] as u16 & 0x07) << 6) | (me[3] as u16 >> 2);
        if baro_raw > 0 { m.nav_qnh = Some((baro_raw as f64 - 1.0) * 0.8 + 800.0); }
        // Heading
        let hdg_valid = (me[3] >> 1) & 1;
        if hdg_valid == 1 {
            let hdg_raw = ((me[3] as u16 & 0x01) << 8) | me[4] as u16;
            m.nav_heading = Some(hdg_raw as f64 * 360.0 / 512.0);
        }
        // NACp
        m.nac_p = Some((me[5] >> 5) & 0x07 | ((me[5] >> 4) & 0x01) << 3);
        m.nic_baro = Some((me[5] >> 3) & 1);
        // Nav modes
        let modes = me[5] & 0x07;
        let modes2 = (me[6] >> 5) & 0x07;
        m.nav_modes = Some((modes << 3) | modes2);
    }
}

/// TC=31: Operational Status
fn decode_operational_status(me: &[u8], m: &mut Message) {
    let subtype = me[0] & 0x07;
    m.adsb_version = Some((me[5] >> 5) & 0x07);
    m.nic = Some(((me[5] >> 3) & 0x03) | ((me[2] >> 2) & 0x04)); // NIC supplement
    m.nac_p = Some(me[5] & 0x0F);
    m.sil = Some((me[6] >> 6) & 0x03);
    m.sil_type = Some((me[6] >> 4) & 0x01);
    m.gva = Some((me[6] >> 2) & 0x03);
    m.sda = Some(me[6] & 0x03);
    if subtype == 0 {
        // Airborne
        m.nic_baro = Some((me[5] >> 4) & 1);
        m.nac_v = Some((me[3] >> 1) & 0x07);
    }
}

/// BDS register decoding for DF20/21 Comm-B replies
/// DISABLED: BDS identification is unreliable without proper heuristics.
/// Bad BDS decodes overwrite good ADS-B values with garbage.
fn decode_bds(_mb: &[u8], _m: &mut Message) {
    // TODO: implement proper BDS identification (BDS 1,7 / 2,0 cross-check)
}

// --- Altitude decoders ---
fn decode_ac12(ac12: u16) -> Option<i32> {
    if ac12 == 0 { return None; }
    let q_bit = (ac12 >> 4) & 1;
    if q_bit == 1 { Some((((ac12 & 0xFE0) >> 1) | (ac12 & 0x00F)) as i32 * 25 - 1000) } else { None }
}
fn decode_ac13(bytes: &[u8]) -> Option<i32> {
    let ac13 = (((bytes[0] as u16) << 8) | bytes[1] as u16) & 0x1FFF;
    if ac13 == 0 { return None; }
    let m_bit = (ac13 >> 6) & 1; let q_bit = (ac13 >> 4) & 1;
    if m_bit == 0 && q_bit == 1 { Some((((ac13 & 0x1F80) >> 2) | ((ac13 & 0x0020) >> 1) | (ac13 & 0x000F)) as i32 * 25 - 1000) } else { None }
}
fn decode_id13(bytes: &[u8]) -> u16 {
    let id13 = (((bytes[0] as u16) << 8) | bytes[1] as u16) & 0x1FFF;
    let a = ((id13 >> 10) & 0x07) as u16;
    let b = ((id13 >> 4) & 0x07) as u16;
    let c = ((id13 >> 1) & 0x07) as u16;
    let d = ((id13 & 0x01) | ((id13 >> 12) & 0x02) | ((id13 >> 7) & 0x04)) as u16;
    a * 1000 + b * 100 + c * 10 + d
}
fn decode_id13_from_raw(sq: u16) -> u16 {
    let a = ((sq >> 10) & 0x07) as u16;
    let b = ((sq >> 4) & 0x07) as u16;
    let c = ((sq >> 1) & 0x07) as u16;
    let d = ((sq & 0x01) | ((sq >> 12) & 0x02) | ((sq >> 7) & 0x04)) as u16;
    a * 1000 + b * 100 + c * 10 + d
}
fn decode_movement(movement: u16) -> Option<f64> {
    if movement == 0 { return None; }
    if movement == 1 { return Some(0.0); }
    Some(match movement {
        2..=8 => (movement as f64 - 1.0) * 0.125,
        9..=12 => 1.0 + (movement as f64 - 9.0) * 0.25,
        13..=38 => 2.0 + (movement as f64 - 13.0) * 0.5,
        39..=93 => 15.0 + (movement as f64 - 39.0),
        94..=108 => 70.0 + (movement as f64 - 94.0) * 2.0,
        109..=123 => 100.0 + (movement as f64 - 109.0) * 5.0,
        124 => 175.0,
        _ => return None,
    })
}

// --- CRC ---
fn crc_residual(msg: &[u8], len: usize) -> u32 {
    let mut crc: u32 = 0;
    for i in 0..(len * 8) {
        let byte_idx = i / 8; let bit_idx = 7 - (i % 8);
        let bit = if byte_idx < msg.len() { (msg[byte_idx] >> bit_idx) & 1 } else { 0 };
        if (crc & 0x800000) != 0 { crc = ((crc << 1) | bit as u32) ^ 0xFFF409; } else { crc = (crc << 1) | bit as u32; }
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
            ias: None, tas: None, mach: None, mag_heading: None, true_heading: None,
            roll: None, track_rate: None, geom_rate: None,
            nav_altitude_mcp: None, nav_altitude_fms: None, nav_qnh: None,
            nav_heading: None, nav_modes: None,
            nic: None, nac_p: None, nac_v: None, sil: None, sil_type: None,
            gva: None, sda: None, nic_baro: None, adsb_version: None,
            emergency: None, addr_type: 0,
        }
    }
}

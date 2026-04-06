use adsb_deku::Frame;
use adsb_deku::adsb::ME;
use adsb_deku::{DF, ICAO, CPRFormat};
use adsb_deku::deku::DekuContainerRead;

use crate::beast::parser::BeastFrame;
use crate::state::{AircraftStore, AircraftUpdate};

fn icao_to_u32(icao: &ICAO) -> u32 {
    ((icao.0[0] as u32) << 16) | ((icao.0[1] as u32) << 8) | (icao.0[2] as u32)
}

pub fn decode_and_update(beast: &BeastFrame, store: &AircraftStore) {
    let frame = match Frame::from_bytes((&beast.payload, 0)) {
        Ok((_, f)) => f,
        Err(_) => return,
    };

    let update = match &frame.df {
        DF::ADSB(adsb) => {
            let icao = icao_to_u32(&adsb.icao);
            let mut u = AircraftUpdate::new(icao);
            u.signal = Some(beast.signal);

            match &adsb.me {
                ME::AirbornePositionBaroAltitude(alt) => {
                    if let Some(a) = alt.alt { u.alt_baro = Some(a as i32); }
                    u.cpr_lat = Some(alt.lat_cpr);
                    u.cpr_lon = Some(alt.lon_cpr);
                    u.cpr_odd = Some(alt.odd_flag == CPRFormat::Odd);
                }
                ME::AirbornePositionGNSSAltitude(alt) => {
                    if let Some(a) = alt.alt { u.alt_geom = Some(a as i32); }
                    u.cpr_lat = Some(alt.lat_cpr);
                    u.cpr_lon = Some(alt.lon_cpr);
                    u.cpr_odd = Some(alt.odd_flag == CPRFormat::Odd);
                }
                ME::AircraftIdentification(id) => {
                    let cs = id.cn.trim().to_string();
                    if !cs.is_empty() { u.callsign = Some(cs); }
                }
                _ => {}
            }
            u
        }
        DF::ShortAirAirSurveillance { altitude, .. } => {
            let icao = frame.crc;
            let mut u = AircraftUpdate::new(icao);
            u.signal = Some(beast.signal);
            if altitude.0 > 0 { u.alt_baro = Some(altitude.0 as i32); }
            u
        }
        DF::SurveillanceAltitudeReply { ac, .. } => {
            let icao = frame.crc;
            let mut u = AircraftUpdate::new(icao);
            u.signal = Some(beast.signal);
            if ac.0 > 0 { u.alt_baro = Some(ac.0 as i32); }
            u
        }
        DF::AllCallReply { icao, .. } => {
            AircraftUpdate::new(icao_to_u32(icao))
        }
        _ => return,
    };

    store.update(update);
}

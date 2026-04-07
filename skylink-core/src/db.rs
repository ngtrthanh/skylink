/// Aircraft database — ICAO hex → type designator, description, WTC

use std::collections::HashMap;
use tracing::info;

pub struct AircraftDb {
    /// hex (lowercase) → (registration, type_designator)
    pub aircraft: HashMap<String, (String, String)>,
    /// type_designator → (description_code, wtc)
    pub types: HashMap<String, (String, String)>,
}

impl AircraftDb {
    pub fn load() -> Self {
        let ac_data = include_str!("../aircrafts.json");
        let types_data = include_str!("../types.json");

        let ac_raw: HashMap<String, Vec<String>> = serde_json::from_str(ac_data).unwrap_or_default();
        let types_raw: HashMap<String, Vec<String>> = serde_json::from_str(types_data).unwrap_or_default();

        let mut aircraft = HashMap::with_capacity(ac_raw.len());
        for (hex, vals) in ac_raw {
            if vals.len() >= 2 {
                let reg = vals[0].clone();
                let t = vals[1].clone();
                if !t.is_empty() {
                    aircraft.insert(hex.to_lowercase(), (reg, t));
                }
            }
        }

        let mut types = HashMap::with_capacity(types_raw.len());
        for (td, vals) in types_raw {
            if vals.len() >= 3 {
                types.insert(td, (vals[1].clone(), vals[2].clone()));
            }
        }

        info!("aircraft db: {} aircraft, {} types", aircraft.len(), types.len());
        AircraftDb { aircraft, types }
    }

    /// Look up type designator for an ICAO hex
    pub fn get_type(&self, hex: &str) -> Option<&str> {
        self.aircraft.get(hex).map(|(_, t)| t.as_str())
    }

    /// Look up type description code and WTC for a type designator
    pub fn get_type_info(&self, type_desig: &str) -> Option<(&str, &str)> {
        self.types.get(type_desig).map(|(d, w)| (d.as_str(), w.as_str()))
    }
}

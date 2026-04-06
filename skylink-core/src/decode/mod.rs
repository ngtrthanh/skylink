pub mod mod_s;

use crate::beast::parser::BeastFrame;
use crate::state::AircraftStore;

pub fn process_frame(frame: &BeastFrame, store: &AircraftStore) {
    mod_s::decode_and_update(frame, store);
}

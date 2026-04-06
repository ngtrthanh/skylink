fn main() {
    prost_build::compile_protos(&["readsb.proto"], &["."]).unwrap();
}

fn main() {
    prost_build::compile_protos(&["proto/save.proto"], &["proto/"]).unwrap();
}

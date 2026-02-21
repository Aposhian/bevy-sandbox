fn main() {
    prost_build::compile_protos(&["proto/save.proto"], &["proto/"]).unwrap();
    tonic_build::compile_protos("proto/network.proto").unwrap();
}

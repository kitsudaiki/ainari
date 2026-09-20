/// Build-script, which generates the grpc-server of the neko from the shared protobuf-definition,
/// so client and server always use the same interface.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../libs/rust/ainari_clients/src/proto/neko_wrapper.proto")?;
    Ok(())
}

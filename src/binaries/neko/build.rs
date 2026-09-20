fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../libs/rust/ainari_clients/src/proto/neko_wrapper.proto")?;
    Ok(())
}

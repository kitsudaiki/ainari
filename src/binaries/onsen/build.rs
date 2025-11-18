fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../libs/rust/ainari_clients/src/proto/onsen_upload.proto")?;
    Ok(())
}

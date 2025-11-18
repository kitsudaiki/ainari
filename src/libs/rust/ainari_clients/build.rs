fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/proto/onsen_upload.proto");

    tonic_build::compile_protos("src/proto/onsen_upload.proto")?;
    Ok(())
}

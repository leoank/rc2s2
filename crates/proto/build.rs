fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_prost_build::configure().compile_protos(
        &[
            "proto/rc2s2/event/v1/event.proto",
            "proto/rc2s2/node/v1/node.proto",
            "proto/rc2s2/control/v1/control.proto",
        ],
        &["proto"],
    )?;
    Ok(())
}

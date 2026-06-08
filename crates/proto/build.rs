fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Proto files live at the project root (../../proto relative to this crate).
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../../proto/auth/v1/auth.proto",
                "../../proto/user/v1/user.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}

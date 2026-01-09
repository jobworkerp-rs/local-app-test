fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tauri build
    tauri_build::build();

    // gRPC proto compilation
    let proto_root = "proto";
    let protos = [
        // Data definitions
        "jobworkerp/data/common.proto",
        "jobworkerp/data/job.proto",
        "jobworkerp/data/job_result.proto",
        "jobworkerp/data/runner.proto",
        "jobworkerp/data/worker.proto",
        // Service definitions
        "jobworkerp/service/common.proto",
        "jobworkerp/service/job.proto",
        "jobworkerp/service/job_result.proto",
        "jobworkerp/service/runner.proto",
        "jobworkerp/service/worker.proto",
    ];

    let proto_files: Vec<String> = protos
        .iter()
        .map(|p| format!("{}/{}", proto_root, p))
        .collect();

    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .out_dir("src/grpc/generated")
        .compile_protos(&proto_files, &[proto_root])?;

    // Rerun if proto files change
    println!("cargo:rerun-if-changed={}", proto_root);

    Ok(())
}

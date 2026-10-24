// Test-only adapter built against the pinned baseline, never the current library.
use std::io::Read;

fn main() {
    let mut bytes = Vec::new();
    std::io::stdin().read_to_end(&mut bytes).unwrap();
    let result =
        serde_json::from_slice::<beta001_harness::service::evidence::ServiceEvidencePackage>(
            &bytes,
        )
        .map_err(|error| error.to_string())
        .and_then(|package| package.verify_chain().map_err(|error| format!("{error:?}")));
    match result {
        Ok(()) => println!("BASELINE_V1_ACCEPTED"),
        Err(error) => {
            eprintln!("BASELINE_V1_REJECTED: {error}");
            std::process::exit(1);
        }
    }
}

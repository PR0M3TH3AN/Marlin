// cli-bin/build.rs
//
// The CLI currently needs no build-time code-generation, but Cargo
// insists on rerunning any build-script each compile.  Tell it to
// rebuild only if this file itself changes.

fn main() {
    // If you later add code-gen (e.g. embed completions or YAML), add
    // further `cargo:rerun-if-changed=<path>` lines here.
    println!("cargo:rerun-if-changed=build.rs");
}

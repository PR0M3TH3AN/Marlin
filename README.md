# 1. Build
cargo build --release

# 2. Initialise DB (idempotent)
./target/release/marlin init

# 3. Scan a directory
./target/release/marlin scan ~/Pictures

# 4. Tag all JPEGs in Pictures
./target/release/marlin tag "~/Pictures/**/*.jpg" vacation

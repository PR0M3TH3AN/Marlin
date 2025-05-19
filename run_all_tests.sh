#!/usr/bin/env bash

# Comprehensive Marlin Test Script
#
# This script will:
# 1. Clean up previous test artifacts.
# 2. Build and install the Marlin CLI.
# 3. Generate a new test corpus and demo directories.
# 4. Run all automated tests (unit, integration, e2e).
# 5. Run benchmark scripts.
# 6. Execute steps from marlin_demo.md.
# 7. Clean up generated test artifacts.

set -euo pipefail # Exit on error, undefined variable, or pipe failure
IFS=$'\n\t'     # Safer IFS

# --- Configuration ---
MARLIN_REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" # Assumes script is in repo root
CARGO_TARGET_DIR_VALUE="${MARLIN_REPO_ROOT}/target" # Consistent target dir

# Test artifact locations
TEST_BASE_DIR="${MARLIN_REPO_ROOT}/_test_artifacts" # Main directory for all test stuff
DEMO_DIR="${TEST_BASE_DIR}/marlin_demo"
CORPUS_DIR_BENCH="${MARLIN_REPO_ROOT}/bench/corpus" # Used by bench scripts
CORPUS_DIR_SCRIPT="${TEST_BASE_DIR}/corpus_generated_by_script" # If script generates its own
TEMP_DB_DIR="${TEST_BASE_DIR}/temp_dbs"

MARLIN_BIN_NAME="marlin"
MARLIN_INSTALL_PATH="/usr/local/bin/${MARLIN_BIN_NAME}" # Adjust if you install elsewhere

# Colors for logging
COLOR_GREEN='\033[0;32m'
COLOR_YELLOW='\033[0;33m'
COLOR_RED='\033[0;31m'
COLOR_BLUE='\033[0;34m'
COLOR_RESET='\033[0m'

# --- Helper Functions ---
log_info() {
    echo -e "${COLOR_GREEN}[INFO]${COLOR_RESET} $1"
}
log_warn() {
    echo -e "${COLOR_YELLOW}[WARN]${COLOR_RESET} $1"
}
log_error() {
    echo -e "${COLOR_RED}[ERROR]${COLOR_RESET} $1" >&2
}
log_section() {
    echo -e "\n${COLOR_BLUE}>>> $1 <<<${COLOR_RESET}"
}

run_cmd() {
    log_info "Executing: $*"
    "$@"
    local status=$?
    if [ $status -ne 0 ]; then
        log_error "Command failed with status $status: $*"
        # exit $status # Optional: exit immediately on any command failure
    fi
    return $status
}

# Trap for cleanup
cleanup_final() {
    log_section "Final Cleanup"
    log_info "Removing test artifacts directory: ${TEST_BASE_DIR}"
    rm -rf "${TEST_BASE_DIR}"
    # Note: bench/corpus might be left as it's part of the repo structure if not deleted by gen-corpus
    # If gen-corpus.sh always creates bench/corpus, then we can remove it here too.
    # For now, let's assume gen-corpus.sh handles its own target.
    if [ -d "${MARLIN_REPO_ROOT}/bench/index.db" ]; then
        log_info "Removing benchmark database: ${MARLIN_REPO_ROOT}/bench/index.db"
        rm -f "${MARLIN_REPO_ROOT}/bench/index.db"
    fi
    if [ -d "${MARLIN_REPO_ROOT}/bench/dirty-vs-full.md" ]; then
        log_info "Removing benchmark report: ${MARLIN_REPO_ROOT}/bench/dirty-vs-full.md"
        rm -f "${MARLIN_REPO_ROOT}/bench/dirty-vs-full.md"
    fi
    log_info "Cleanup complete."
}
trap 'cleanup_final' EXIT INT TERM

# --- Test Functions ---

initial_cleanup_and_setup_dirs() {
    log_section "Initial Cleanup and Directory Setup"
    if [ -d "${TEST_BASE_DIR}" ]; then
        log_info "Removing previous test artifacts: ${TEST_BASE_DIR}"
        rm -rf "${TEST_BASE_DIR}"
    fi
    run_cmd mkdir -p "${DEMO_DIR}"
    run_cmd mkdir -p "${CORPUS_DIR_SCRIPT}"
    run_cmd mkdir -p "${TEMP_DB_DIR}"
    log_info "Test directories created under ${TEST_BASE_DIR}"

    # Cleanup existing benchmark corpus if gen-corpus.sh is expected to always create it
    if [ -d "${CORPUS_DIR_BENCH}" ]; then
        log_info "Removing existing benchmark corpus: ${CORPUS_DIR_BENCH}"
        rm -rf "${CORPUS_DIR_BENCH}"
    fi
    if [ -f "${MARLIN_REPO_ROOT}/bench/index.db" ]; then
        rm -f "${MARLIN_REPO_ROOT}/bench/index.db"
    fi
    if [ -f "${MARLIN_REPO_ROOT}/bench/dirty-vs-full.md" ]; then
        rm -f "${MARLIN_REPO_ROOT}/bench/dirty-vs-full.md"
    fi
}

build_and_install_marlin() {
    log_section "Building and Installing Marlin"
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR_VALUE}"
    log_info "CARGO_TARGET_DIR set to ${CARGO_TARGET_DIR}"

    run_cmd cargo build --release --manifest-path "${MARLIN_REPO_ROOT}/cli-bin/Cargo.toml"
    
    COMPILED_MARLIN_BIN="${CARGO_TARGET_DIR_VALUE}/release/${MARLIN_BIN_NAME}"
    if [ ! -f "${COMPILED_MARLIN_BIN}" ]; then
        log_error "Marlin binary not found at ${COMPILED_MARLIN_BIN} after build!"
        exit 1
    fi

    log_info "Installing Marlin to ${MARLIN_INSTALL_PATH} (requires sudo)"
    run_cmd sudo install -Dm755 "${COMPILED_MARLIN_BIN}" "${MARLIN_INSTALL_PATH}"
    # Alternative without sudo (if MARLIN_INSTALL_PATH is in user's PATH):
    # run_cmd cp "${COMPILED_MARLIN_BIN}" "${MARLIN_INSTALL_PATH}"
    # run_cmd chmod +x "${MARLIN_INSTALL_PATH}"
    log_info "Marlin installed."
    run_cmd "${MARLIN_INSTALL_PATH}" --version # Verify installation
}

generate_test_data() {
    log_section "Generating Test Data"

    log_info "Generating benchmark corpus..."
    # Ensure gen-corpus.sh targets the correct directory if it's configurable
    # Current gen-corpus.sh targets bench/corpus
    run_cmd bash "${MARLIN_REPO_ROOT}/bench/gen-corpus.sh"
    # If you want a separate corpus for other tests:
    # COUNT=100 TARGET="${CORPUS_DIR_SCRIPT}" run_cmd bash "${MARLIN_REPO_ROOT}/bench/gen-corpus.sh"

    log_info "Setting up marlin_demo tree in ${DEMO_DIR}"
    mkdir -p "${DEMO_DIR}"/{Projects/{Alpha,Beta,Gamma},Logs,Reports,Scripts,Media/Photos}

    cat <<EOF > "${DEMO_DIR}/Projects/Alpha/draft1.md"
# Alpha draft 1
- [ ] TODO: outline architecture
- [ ] TODO: write tests
EOF
    cat <<EOF > "${DEMO_DIR}/Projects/Alpha/draft2.md"
# Alpha draft 2
- [x] TODO: outline architecture
- [ ] TODO: implement feature X
EOF
    cat <<EOF > "${DEMO_DIR}/Projects/Beta/notes.md"
Beta meeting notes:

- decided on roadmap
- ACTION: follow-up with design team
EOF
    cat <<EOF > "${DEMO_DIR}/Projects/Beta/final.md"
# Beta Final
All tasks complete. Ready to ship!
EOF
    cat <<EOF > "${DEMO_DIR}/Projects/Gamma/TODO.txt"
Gamma tasks:
TODO: refactor module Y
EOF
    echo "2025-05-15 12:00:00 INFO  Starting app"  >  "${DEMO_DIR}/Logs/app.log"
    echo "2025-05-15 12:01:00 ERROR Oops, crash"   >> "${DEMO_DIR}/Logs/app.log"
    echo "2025-05-15 00:00:00 INFO  System check OK" > "${DEMO_DIR}/Logs/system.log"
    printf "Q1 financials\n" > "${DEMO_DIR}/Reports/Q1_report.pdf"
    cat <<'EOSH' > "${DEMO_DIR}/Scripts/deploy.sh"
#!/usr/bin/env bash
echo "Deploying version $1…"
EOSH
    chmod +x "${DEMO_DIR}/Scripts/deploy.sh"
    echo "JPEGDATA" > "${DEMO_DIR}/Media/Photos/event.jpg"
    log_info "marlin_demo tree created."
}

run_cargo_tests() {
    log_section "Running Cargo Tests (Unit & Integration)"
    export CARGO_TARGET_DIR="${CARGO_TARGET_DIR_VALUE}" # Ensure it's set for test context too

    run_cmd cargo test --all --manifest-path "${MARLIN_REPO_ROOT}/Cargo.toml" -- --nocapture
    # Individual test suites (already covered by --all, but can be run specifically)
    # run_cmd cargo test --test e2e --manifest-path "${MARLIN_REPO_ROOT}/cli-bin/Cargo.toml" -- --nocapture
    # run_cmd cargo test --test pos --manifest-path "${MARLIN_REPO_ROOT}/cli-bin/Cargo.toml" -- --nocapture
    # run_cmd cargo test --test neg --manifest-path "${MARLIN_REPO_ROOT}/cli-bin/Cargo.toml" -- --nocapture
    # run_cmd cargo test --test watcher_test --manifest-path "${MARLIN_REPO_ROOT}/cli-bin/Cargo.toml" -- --nocapture
    log_info "Cargo tests complete."
}

run_benchmarks() {
    log_section "Running Benchmark Scripts"
    if ! command -v hyperfine &> /dev/null; then
        log_warn "hyperfine command not found. Skipping dirty-vs-full benchmark."
        return
    fi
    # Ensure MARLIN_BIN is set for the script, pointing to our freshly installed one or compiled one
    export MARLIN_BIN="${MARLIN_INSTALL_PATH}"
    # Or, if not installing system-wide:
    # export MARLIN_BIN="${CARGO_TARGET_DIR_VALUE}/release/${MARLIN_BIN_NAME}"
    
    # The script itself sets MARLIN_DB_PATH to bench/index.db
    run_cmd bash "${MARLIN_REPO_ROOT}/bench/dirty-vs-full.sh"
    log_info "Benchmark script complete. Results in bench/dirty-vs-full.md"
}

test_tui_stub() {
    log_section "Testing TUI Stub"
    local tui_bin="${CARGO_TARGET_DIR_VALUE}/release/marlin-tui"
    if [ ! -f "${tui_bin}" ]; then
        log_warn "Marlin TUI binary not found at ${tui_bin}. Building..."
        run_cmd cargo build --release --manifest-path "${MARLIN_REPO_ROOT}/tui-bin/Cargo.toml"
    fi
    
    if [ -f "${tui_bin}" ]; then
        log_info "Running TUI stub..."
        # Check for expected output
        output=$("${tui_bin}" 2>&1)
        expected_output="marlin-tui is not yet implemented. Stay tuned!"
        if [[ "$output" == *"$expected_output"* ]]; then
            log_info "TUI stub output is correct."
        else
            log_error "TUI stub output mismatch. Expected: '$expected_output', Got: '$output'"
        fi
    else
        log_error "Marlin TUI binary still not found after attempt to build. Skipping TUI stub test."
    fi
}

test_marlin_demo_flow() {
    log_section "Testing Marlin Demo Flow (docs/marlin_demo.md)"
    # This function will execute the commands from marlin_demo.md
    # It uses the MARLIN_INSTALL_PATH, assumes `marlin` is in PATH due to install
    # The demo uses a DB at DEMO_DIR/index.db by running init from DEMO_DIR

    local marlin_cmd="${MARLIN_INSTALL_PATH}" # or just "marlin" if PATH is set
    local original_dir=$(pwd)

    # Create a specific DB for this demo test, isolated from others
    local demo_db_path="${DEMO_DIR}/.marlin_index_demo.db"
    export MARLIN_DB_PATH="${demo_db_path}"
    log_info "Using demo-specific DB: ${MARLIN_DB_PATH}"

    cd "${DEMO_DIR}" # Critical: init scans CWD

    log_info "Running: ${marlin_cmd} init"
    run_cmd "${marlin_cmd}" init

    log_info "Running tagging commands..."
    run_cmd "${marlin_cmd}" tag "${DEMO_DIR}/Projects/**/*.md" project/md
    run_cmd "${marlin_cmd}" tag "${DEMO_DIR}/Logs/**/*.log" logs/app
    run_cmd "${marlin_cmd}" tag "${DEMO_DIR}/Projects/Beta/**/*" project/beta

    log_info "Running attribute commands..."
    run_cmd "${marlin_cmd}" attr set "${DEMO_DIR}/Projects/Beta/final.md" status complete
    run_cmd "${marlin_cmd}" attr set "${DEMO_DIR}/Reports/*.pdf" reviewed yes

    log_info "Running search commands..."
    run_cmd "${marlin_cmd}" search TODO | grep "TODO.txt" || (log_error "Search TODO failed"; exit 1)
    run_cmd "${marlin_cmd}" search tag:project/md | grep "draft1.md" || (log_error "Search tag:project/md failed"; exit 1)
    run_cmd "${marlin_cmd}" search 'tag:logs/app AND ERROR' | grep "app.log" || (log_error "Search logs/app AND ERROR failed"; exit 1)
    run_cmd "${marlin_cmd}" search 'attr:status=complete' | grep "final.md" || (log_error "Search attr:status=complete failed"; exit 1)
    # Skipping --exec for automated script to avoid opening GUI
    # run_cmd "${marlin_cmd}" search 'attr:reviewed=yes' --exec 'echo {}'

    log_info "Running backup and restore..."
    snap_output=$(run_cmd "${marlin_cmd}" backup)
    snap_file=$(echo "${snap_output}" | awk '{print $NF}')
    log_info "Backup created: ${snap_file}"
    
    if [ -z "${MARLIN_DB_PATH}" ]; then
        log_error "MARLIN_DB_PATH is not set, cannot simulate disaster for restore test."
    elif [ ! -f "${MARLIN_DB_PATH}" ]; then
        log_error "MARLIN_DB_PATH (${MARLIN_DB_PATH}) does not point to a file."
    else
        log_info "Simulating disaster: removing ${MARLIN_DB_PATH}"
        rm -f "${MARLIN_DB_PATH}"
        # Also remove WAL/SHM files if they exist
        rm -f "${MARLIN_DB_PATH}-wal"
        rm -f "${MARLIN_DB_PATH}-shm"

        log_info "Restoring from ${snap_file}"
        run_cmd "${marlin_cmd}" restore "${snap_file}"
        run_cmd "${marlin_cmd}" search TODO | grep "TODO.txt" || (log_error "Search TODO after restore failed"; exit 1)
    fi


    log_info "Running linking demo..."
    touch "${DEMO_DIR}/foo.txt" "${DEMO_DIR}/bar.txt"
    run_cmd "${marlin_cmd}" scan "${DEMO_DIR}" # Index new files

    local foo_path="${DEMO_DIR}/foo.txt"
    local bar_path="${DEMO_DIR}/bar.txt"
    run_cmd "${marlin_cmd}" link add "${foo_path}" "${bar_path}" --type references
    run_cmd "${marlin_cmd}" link list "${foo_path}" | grep "bar.txt" || (log_error "Link list failed"; exit 1)
    run_cmd "${marlin_cmd}" link backlinks "${bar_path}" | grep "foo.txt" || (log_error "Link backlinks failed"; exit 1)

    log_info "Running collections & smart views demo..."
    run_cmd "${marlin_cmd}" coll create SetA
    run_cmd "${marlin_cmd}" coll add SetA "${DEMO_DIR}/Projects/**/*.md"
    run_cmd "${marlin_cmd}" coll list SetA | grep "draft1.md" || (log_error "Coll list failed"; exit 1)

    run_cmd "${marlin_cmd}" view save tasks 'attr:status=complete OR TODO'
    run_cmd "${marlin_cmd}" view exec tasks | grep "final.md" || (log_error "View exec tasks failed"; exit 1)

    unset MARLIN_DB_PATH # Clean up env var for this specific test
    cd "${original_dir}"
    log_info "Marlin Demo Flow test complete."
}

test_backup_prune_cli() {
    log_section "Testing Backup Pruning (CLI)"
    # This test assumes `marlin backup --prune N` is implemented in the CLI.
    # If not, it will likely fail or this section should be marked TODO.

    local marlin_cmd="${MARLIN_INSTALL_PATH}"
    local backup_test_db_dir="${TEMP_DB_DIR}/backup_prune_test"
    mkdir -p "${backup_test_db_dir}"
    local test_db="${backup_test_db_dir}/test_prune.db"
    export MARLIN_DB_PATH="${test_db}"
    
    log_info "Initializing DB for prune test at ${test_db}"
    run_cmd "${marlin_cmd}" init # Run from CWD to init DB at MARLIN_DB_PATH

    local backup_storage_dir="${backup_test_db_dir}/backups" # Marlin creates backups next to the DB by default

    log_info "Creating multiple backups..."
    for i in {1..7}; do
        run_cmd "${marlin_cmd}" backup > /dev/null # Suppress output for cleaner logs
        sleep 0.1 # Ensure unique timestamps if backups are very fast
    done

    local num_backups_before_prune=$(ls -1 "${backup_storage_dir}" | grep -c "backup_.*\.db$" || echo 0)
    log_info "Number of backups before prune: ${num_backups_before_prune}"
    if [ "${num_backups_before_prune}" -lt 7 ]; then
        log_warn "Expected at least 7 backups, found ${num_backups_before_prune}. Prune test might be less effective."
    fi
    
    # Check if `marlin backup --prune` exists in help output.
    # This is a basic check for CLI command availability.
    if ! "${marlin_cmd}" backup --help | grep -q "\-\-prune"; then
        log_warn "marlin backup --prune N does not seem to be an available CLI option."
        log_warn "Skipping CLI backup prune test. Implement it or update this test."
        unset MARLIN_DB_PATH
        return
    fi
    
    log_info "Running: ${marlin_cmd} backup --prune 3"
    run_cmd "${marlin_cmd}" backup --prune 3 # This should create one more backup, then prune
                                         # leaving 3 newest (including the one just made).

    local num_backups_after_prune=$(ls -1 "${backup_storage_dir}" | grep -c "backup_.*\.db$" || echo 0)
    log_info "Number of backups after prune: ${num_backups_after_prune}"

    if [ "${num_backups_after_prune}" -eq 3 ]; then
        log_info "Backup prune CLI test successful: 3 backups remaining."
    else
        log_error "Backup prune CLI test FAILED: Expected 3 backups, found ${num_backups_after_prune}."
    fi
    unset MARLIN_DB_PATH
}

test_watcher_cli_basic() {
    log_section "Testing Watcher CLI Basic Operation (Short Test)"
    # This is a very basic, short-running test for `marlin watch start`
    # A full stress test (8h) is a separate, longer process.

    local marlin_cmd="${MARLIN_INSTALL_PATH}"
    local watch_test_dir="${TEMP_DB_DIR}/watch_cli_test_data"
    local watch_test_db="${TEMP_DB_DIR}/watch_cli_test.db"
    mkdir -p "${watch_test_dir}"
    export MARLIN_DB_PATH="${watch_test_db}"

    log_info "Initializing DB for watcher test at ${watch_test_db}"
    run_cmd "${marlin_cmd}" init # Run from CWD for init

    log_info "Starting watcher in background for 10 seconds..."
    # Run watcher in a subshell and kill it. Redirect output to a log file.
    local watcher_log="${TEST_BASE_DIR}/watcher_cli.log"
    ( cd "${watch_test_dir}" && timeout 10s "${marlin_cmd}" watch start . --debounce-ms 50 &> "${watcher_log}" ) &
    local watcher_pid=$!
    
    # Give watcher a moment to start
    sleep 2 

    log_info "Creating and modifying files in watched directory: ${watch_test_dir}"
    touch "${watch_test_dir}/file_created.txt"
    sleep 0.2
    echo "modified" > "${watch_test_dir}/file_created.txt"
    sleep 0.2
    mkdir "${watch_test_dir}/subdir"
    touch "${watch_test_dir}/subdir/file_in_subdir.txt"
    sleep 0.2
    rm "${watch_test_dir}/file_created.txt"
    
    log_info "Waiting for watcher process (PID ${watcher_pid}) to finish (max 10s timeout)..."
    # wait ${watcher_pid} # This might hang if timeout doesn't kill cleanly
    # Instead, rely on the `timeout` command or send SIGINT/SIGTERM if needed.
    # For this test, the timeout command handles termination.
    # We need to ensure the watcher has time to process events before it's killed.
    sleep 5 # Allow time for events to be processed by the watcher

    # The timeout should have killed the watcher. If not, try to kill it.
    if ps -p ${watcher_pid} > /dev/null; then
       log_warn "Watcher process ${watcher_pid} still running after timeout. Attempting to kill."
       kill ${watcher_pid} || true
       sleep 1
       kill -9 ${watcher_pid} || true
    fi

    log_info "Watcher process should have terminated."
    log_info "Checking watcher log: ${watcher_log}"
    if [ -f "${watcher_log}" ]; then
        cat "${watcher_log}" # Display the log for debugging
        # Example checks on the log (these are basic, can be more specific)
        grep -q "CREATE" "${watcher_log}" && log_info "CREATE event found in log." || log_warn "CREATE event NOT found in log."
        grep -q "MODIFY" "${watcher_log}" && log_info "MODIFY event found in log." || log_warn "MODIFY event NOT found in log."
        grep -q "REMOVE" "${watcher_log}" && log_info "REMOVE event found in log." || log_warn "REMOVE event NOT found in log."
    else
        log_error "Watcher log file not found: ${watcher_log}"
    fi

    # TODO: Add verification of DB state after watcher (e.g., file_changes table, new files indexed)
    # This would require querying the DB: sqlite3 "${watch_test_db}" "SELECT * FROM files;"

    unset MARLIN_DB_PATH
    log_info "Watcher CLI basic test complete."
}


# --- Main Execution ---
main() {
    log_section "Starting Marlin Comprehensive Test Suite"
    cd "${MARLIN_REPO_ROOT}" # Ensure we are in the repo root

    initial_cleanup_and_setup_dirs
    build_and_install_marlin
    generate_test_data

    run_cargo_tests
    run_benchmarks
    test_tui_stub
    test_marlin_demo_flow
    test_backup_prune_cli # Add more specific tests here
    test_watcher_cli_basic

    # --- Add new test functions here ---
    # test_new_feature_x() {
    #   log_section "Testing New Feature X"
    #   # ... your test commands ...
    # }
    # test_new_feature_x

    log_section "All Tests Executed"
    log_info "Review logs for any warnings or errors."
}

# Run main
main

# Cleanup is handled by the trap
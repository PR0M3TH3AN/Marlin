// cli-bin/build.rs
//
// Build script to generate the CLI cheatsheet at compile time.  It
// parses `src/cli/commands.yaml` and emits a simple Markdown table of
// commands and flags to `cli-bin/docs/cli_cheatsheet.md`.

use std::{fs, path::Path};

use serde_yaml::Value;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/cli/commands.yaml");

    if let Err(e) = generate_cheatsheet() {
        eprintln!("Failed to generate CLI cheatsheet: {e}");
        std::process::exit(1);
    }
}

fn generate_cheatsheet() -> Result<(), Box<dyn std::error::Error>> {
    let yaml_str = fs::read_to_string("src/cli/commands.yaml")?;
    let parsed: Value = serde_yaml::from_str(&yaml_str)?;

    let mut table = String::from("| Command | Flags |\n| ------- | ----- |\n");

    if let Value::Mapping(cmds) = parsed {
        for (cmd_name_val, cmd_details_val) in cmds {
            let cmd_name = cmd_name_val.as_str().unwrap_or("");
            if let Value::Mapping(cmd_details) = cmd_details_val {
                if let Some(Value::Mapping(actions)) =
                    cmd_details.get(Value::String("actions".into()))
                {
                    for (action_name_val, action_body_val) in actions {
                        let action_name = action_name_val.as_str().unwrap_or("");
                        let flags = if let Value::Mapping(action_map) = action_body_val {
                            match action_map.get(Value::String("flags".into())) {
                                Some(Value::Sequence(seq)) => seq
                                    .iter()
                                    .filter_map(|v| v.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                _ => String::new(),
                            }
                        } else {
                            String::new()
                        };

                        let flags_disp = if flags.is_empty() { "—" } else { &flags };
                        table.push_str(&format!(
                            "| `{} {}` | {} |\n",
                            cmd_name, action_name, flags_disp
                        ));
                    }
                }
            }
        }
    }

    fs::create_dir_all(Path::new("docs"))?;
    fs::write("docs/cli_cheatsheet.md", table)?;

    Ok(())
}

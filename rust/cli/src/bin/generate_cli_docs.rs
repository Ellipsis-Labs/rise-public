use std::error::Error;
use std::fs;
use std::path::Path;

use clap::Command;

const README_START: &str = "<!-- phoenix-rise-cli-command-docs:start -->";
const README_END: &str = "<!-- phoenix-rise-cli-command-docs:end -->";

// Keep the README focused on common workflows. The full CLI surface is still
// available through `phoenix-rise --help` and nested subcommand help.
const README_COMMAND_PATHS: &[&[&str]] = &[
    &[],
    &["auth", "login"],
    &["auth", "session"],
    &["exchange", "keys"],
    &["exchange", "build-register-ixs"],
    &["market", "list"],
    &["market", "show"],
    &["market", "orderbook"],
    &["market", "candles"],
    &["rpc", "account"],
    &["trader", "register"],
    &["trader", "summary"],
    &["trader", "trade-history"],
    &["trader", "set-position-authority"],
    &["trader", "reset-position-authority"],
    &["trader", "place-market-order"],
    &["flight", "view"],
    &["flight", "withdraw-collateral"],
    &["tx", "parse"],
    &["ws", "trader-state"],
];

fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut command = phoenix_rise_cli::command();
    disable_synthetic_help_subcommands(&mut command);
    command.build();

    update_readme(&manifest_dir.join("README.md"), &command)?;

    println!("updated {}", manifest_dir.join("README.md").display());
    Ok(())
}

fn update_readme(readme_path: &Path, command: &Command) -> Result<(), Box<dyn Error>> {
    let readme = fs::read_to_string(readme_path)?;
    let generated = readme_command_reference(command)?;

    let start = readme
        .find(README_START)
        .ok_or("README is missing phoenix-rise-cli command docs start marker")?;
    let end = readme
        .find(README_END)
        .ok_or("README is missing phoenix-rise-cli command docs end marker")?;

    let mut updated = String::with_capacity(readme.len() + generated.len());
    updated.push_str(&readme[..start + README_START.len()]);
    updated.push('\n');
    updated.push_str(&generated);
    if !generated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&readme[end..]);

    fs::write(readme_path, updated)?;
    Ok(())
}

fn readme_command_reference(command: &Command) -> Result<String, Box<dyn Error>> {
    let mut out = String::new();
    out.push_str("Generated from the `phoenix-rise` clap command tree. Regenerate with:\n\n");
    out.push_str("```bash\n");
    out.push_str("cargo run -p phoenix-rise-cli --bin generate-cli-docs\n");
    out.push_str("```\n\n");

    out.push_str(
        "This section shows help for common commands. The CLI supports additional commands and \
         flags; run `phoenix-rise --help`, `phoenix-rise <group> --help`, or `phoenix-rise \
         <group> <command> --help` for the full surface.\n\n",
    );

    append_heading(&mut out, 3, "Selected Command Help");
    for path in README_COMMAND_PATHS {
        append_command_help(&mut out, command, path)?;
    }
    Ok(out)
}

fn append_heading(out: &mut String, level: usize, text: &str) {
    out.push_str(&"#".repeat(level));
    out.push(' ');
    out.push_str(text);
    out.push_str("\n\n");
}

fn append_command_help(
    out: &mut String,
    root: &Command,
    path: &[&str],
) -> Result<(), Box<dyn Error>> {
    let command = command_at_path(root, path)?;
    let full_path = full_command_path(path);
    out.push_str("#### `");
    out.push_str(&full_path);
    out.push_str("`\n\n");
    if let Some(about) = short_about(command) {
        out.push_str(&about);
        out.push_str("\n\n");
    }

    let mut help_command = command.clone().bin_name(full_path);
    let help = trim_trailing_whitespace(&help_command.render_long_help().to_string());
    out.push_str("```text\n");
    out.push_str(&help);
    out.push_str("\n```\n\n");
    Ok(())
}

fn command_at_path<'a>(root: &'a Command, path: &[&str]) -> Result<&'a Command, Box<dyn Error>> {
    let mut command = root;
    for segment in path {
        command = command
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == *segment)
            .ok_or_else(|| format!("unknown phoenix-rise CLI command path: {path:?}"))?;
    }
    Ok(command)
}

fn full_command_path(path: &[&str]) -> String {
    let mut full_path = String::from("phoenix-rise");
    for segment in path {
        full_path.push(' ');
        full_path.push_str(segment);
    }
    full_path
}

fn trim_trailing_whitespace(contents: &str) -> String {
    let mut trimmed = String::with_capacity(contents.len());
    for line in contents.lines() {
        trimmed.push_str(line.trim_end());
        trimmed.push('\n');
    }
    trimmed.trim_end().to_string()
}

fn short_about(command: &Command) -> Option<String> {
    command
        .get_about()
        .or_else(|| command.get_long_about())
        .map(|text| text.to_string())
        .map(|text| text.trim().trim_end_matches('.').to_string())
        .filter(|text| !text.is_empty())
}

fn disable_synthetic_help_subcommands(command: &mut Command) {
    *command = command.clone().disable_help_subcommand(true);
    for subcommand in command.get_subcommands_mut() {
        disable_synthetic_help_subcommands(subcommand);
    }
}

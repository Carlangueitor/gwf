use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
mod commands;

use commands::nfb::{new_branch, prompt_user};
use commands::finish::finish;


#[derive(Debug, Serialize, Deserialize)]
struct Config {
    post_commit_command: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("git-workflow")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("nfb")
                .about("Create a new feature branch with a conventional commit message")
                .arg(Arg::new("type").short('t').long("type").value_name("TYPE").help("Type of the commit (e.g., feat, fix)"))
                .arg(Arg::new("scope").short('s').long("scope").value_name("SCOPE").help("Scope of the commit (e.g., ui, api)"))
                .arg(Arg::new("message").short('m').long("message").value_name("MESSAGE").help("Message for the commit")),
        )
        .subcommand(
            Command::new("finish")
                .about("Commit changes and run a post-commit command"),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("nfb", sub_matches)) => {
            let type_ = sub_matches.get_one::<String>("type").cloned().unwrap_or_else(|| prompt_user("Enter the type of the commit (e.g., feat, fix): "));
            let scope = sub_matches.get_one::<String>("scope").cloned().unwrap_or_else(|| prompt_user("Enter the scope of the commit (e.g., ui, api): "));
            let message = sub_matches.get_one::<String>("message").cloned().unwrap_or_else(|| prompt_user("Enter the message for the commit: "));

            new_branch(&type_, &scope, &message)?;
        }
        Some(("finish", _)) => {
            finish()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

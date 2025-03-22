use clap::{Arg, Command};
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command as ExternalCommand;
use slug::slugify;

const GWF_DIR: &str = ".gwf";
const GWF_CONFIG: &str = "gwf.toml";

fn get_gwf_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(GWF_DIR)
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    post_commit_command: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("git-workflow")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("new")
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
        Some(("new", sub_matches)) => {
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

fn new_branch(type_: &str, scope: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(".")?;
    let branch_name = format!("{}/{}/{}", slugify(type_), slugify(scope), slugify(message));

    // Get the current HEAD commit
    let head = repo.head()?;
    let parent = repo.find_commit(head.target().unwrap())?;

    // Create the new branch
    repo.branch(&branch_name, &parent, false)?;

    // Check out the new branch
    let refname = format!("refs/heads/{}", branch_name);
    repo.set_head(&refname)?;

    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts
        .safe() // Use safe checkout instead of force
        .recreate_missing(true) // Recreate missing files
        .allow_conflicts(true); // Allow conflicts to be resolved later

    repo.checkout_head(Some(&mut checkout_opts))?;

    // Store the commit message in a file outside the repository
    let config_file = get_gwf_dir().join(slugify(&branch_name));
    fs::create_dir_all(config_file.parent().unwrap())?;
    let mut file = fs::File::create(config_file)?;
    writeln!(file, "{}", message)?;

    println!("Branch created and checked out: {}", branch_name);
    Ok(())
}

fn finish() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(".")?;
    let mut index = repo.index()?;

    // Write the current index state to a tree
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    let head = repo.head()?;
    let parent = repo.find_commit(head.target().unwrap())?;

    // Get current branch name and read commit message from file
    let current_branch = head.shorthand().ok_or("Could not get current branch name")?;
    let message_file = get_gwf_dir().join(slugify(current_branch));
    let message = fs::read_to_string(message_file)?;

    // Extract type and scope from branch name (format: type/scope/message)
    let parts: Vec<&str> = current_branch.split('/').collect();
    if parts.len() != 3 {
        return Err("Invalid branch name format. Expected: type/scope/message".into());
    }
    let type_ = parts[0];
    let scope = parts[1];
    
    // Construct conventional commit message
    let commit_message = format!("{}({}): {}", type_, scope, message);

    // Create the commit
    let commit_id = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &commit_message,
        &tree,
        &[&parent]
    )?;

    println!("Created commit: {}", commit_id);

    // Try to read and execute the post-commit command if config exists
    // First try repository root, then .gwf directory
    let repo_root = repo.workdir().ok_or("Could not get repository root")?;
    let config_file = repo_root.join(GWF_CONFIG);
    let config_content = if config_file.exists() {
        fs::read_to_string(config_file)
    } else {
        fs::read_to_string(get_gwf_dir().join(GWF_CONFIG))
    };

    if let Ok(config_content) = config_content {
        if let Ok(config) = toml::from_str::<Config>(&config_content) {
            // Run the post-commit command
            let output = ExternalCommand::new(&config.post_commit_command).output()?;
            if output.status.success() {
                println!("Post-commit command executed successfully");
            } else {
                eprintln!("Post-commit command failed: {}", String::from_utf8_lossy(&output.stderr));
            }
        }
    }

    Ok(())
}

fn prompt_user(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

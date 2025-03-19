use clap::{Arg, Command};
use git2::Repository;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command as ExternalCommand;
use slug::slugify;

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

            create_branch(&type_, &scope, &message)?;
        }
        Some(("finish", _)) => {
            commit_and_run_post_commit_command()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn create_branch(type_: &str, scope: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(".")?;
    let branch_name = format!("{}/{}/{}", slugify(type_), slugify(scope), slugify(message));

    // Create the new branch
    let mut index = repo.index()?;
    index.add_all(["."], git2::IndexAddOption::empty(), None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    let head = repo.head()?;
    let parent = repo.find_commit(head.target().unwrap())?;

    let branch = repo.branch(&branch_name, &parent, false)?;

    // Check out the new branch
    let refname = format!("refs/heads/{}", branch_name);
    repo.set_head(&refname);

    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.force();

    repo.checkout_head(Some(&mut checkout_opts))?;

    // Store the commit message in a file outside the repository
    let config_file = PathBuf::from(format!("~/.git-workflow/{}.txt", branch_name));
    fs::create_dir_all(config_file.parent().unwrap())?;
    let mut file = fs::File::create(config_file)?;
    writeln!(file, "{}", message)?;

    println!("Branch created and checked out: {}", branch_name);
    Ok(())
}

fn commit_and_run_post_commit_command() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(".")?;
    let mut index = repo.index()?;
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = repo.signature()?;
    let head = repo.head()?;
    let parent = repo.find_commit(head.target().unwrap())?;

    // Check if there are any changes in the index
    if index.is_empty() {
        return Err("No changes to commit".into());
    }

    // Commit the changes
    let commit_message = "Conventional commit message".to_string(); // You can modify this to read from a file if needed
    let commit_id = repo.commit(None, &sig, &sig, &commit_message, &tree, &[&parent])?;
    let commit = repo.find_commit(commit_id)?;

    // Read the post-commit command from the user's home directory
    let home_dir = dirs::home_dir().unwrap();
    let config_file = home_dir.join(".git-workflow/config.toml");
    let config_content = fs::read_to_string(config_file)?;
    let config: Config = toml::from_str(&config_content)?;

    // Run the post-commit command
    let output = ExternalCommand::new(&config.post_commit_command).output()?;
    if output.status.success() {
        println!("Post-commit command executed successfully");
    } else {
        eprintln!("Post-commit command failed: {}", String::from_utf8_lossy(&output.stderr));
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

use git2::Repository;
use std::fs;
use std::path::PathBuf;
use std::process::Command as ExternalCommand;
use slug::slugify;
use serde::{Deserialize, Serialize};

const GWF_DIR: &str = ".gwf";
const GWF_CONFIG: &str = "gwf.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    post_commit_command: String,
}

fn get_gwf_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(GWF_DIR)
}

pub fn finish() -> Result<(), Box<dyn std::error::Error>> {
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

    // Extract type and scope from branch name (format: type/scope/message or type/message)
    let parts: Vec<&str> = current_branch.split('/').collect();
    let (type_, scope) = if parts.len() == 2 {
        (parts[0], "")
    } else if parts.len() == 3 {
        (parts[0], parts[1])
    } else {
        return Err("Invalid branch name format. Expected: type/scope/message or type/message".into());
    };
    
    // Construct conventional commit message
    let commit_message = if scope.is_empty() {
        format!("{}: {}", type_, message)
    } else {
        format!("{}({}): {}", type_, scope, message)
    };

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
            let output = ExternalCommand::new("sh")
                .arg("-c")
                .arg(&config.post_commit_command)
                .output()?;
            
            // Print stdout if not empty
            if !output.stdout.is_empty() {
                println!("Post-commit command output:\n{}", String::from_utf8_lossy(&output.stdout));
            }
            
            // Print stderr if not empty
            if !output.stderr.is_empty() {
                eprintln!("Post-commit command errors:\n{}", String::from_utf8_lossy(&output.stderr));
            }
            
            if output.status.success() {
                println!("Post-commit command executed successfully");
            } else {
                eprintln!("Post-commit command failed with exit code: {}", output.status.code().unwrap_or(-1));
            }
        }
    }

    Ok(())
} 
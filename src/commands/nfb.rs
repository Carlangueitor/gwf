use git2::Repository;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use slug::slugify;
use dialoguer::FuzzySelect;

// Common conventional commit types
const CONVENTIONAL_TYPES: &[&str] = &[
    "feat",     // New feature
    "fix",      // Bug fix
    "docs",     // Documentation changes
    "style",    // Code style changes (formatting, etc.)
    "refactor", // Code refactoring
    "perf",     // Performance improvements
    "test",     // Adding or modifying tests
    "build",    // Build system or external dependencies
    "ci",       // CI configuration changes
    "chore",    // Other changes that don't modify source or test files
];

const GWF_DIR: &str = ".gwf";

fn get_gwf_dir() -> PathBuf {
    dirs::home_dir().unwrap().join(GWF_DIR)
}

pub fn new_branch(type_: &str, scope: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    let repo = Repository::open(".")?;
    let branch_name = if scope.is_empty() {
        format!("{}/{}", slugify(type_), slugify(message))
    } else {
        format!("{}/{}/{}", slugify(type_), slugify(scope), slugify(message))
    };

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

pub fn prompt_user(prompt: &str) -> String {
    if prompt.contains("type of the commit") {
        let selection = FuzzySelect::new()
            .with_prompt(prompt)
            .items(CONVENTIONAL_TYPES)
            .interact()
            .unwrap();
        CONVENTIONAL_TYPES[selection].to_string()
    } else {
        print!("{}", prompt);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        input.trim().to_string()
    }
} 
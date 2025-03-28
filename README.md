# git work flow
My custom git work flow packged as a program. It probably won't work for anyone else.

## Usage
```bash
gwf
Usage: gwf <COMMAND>

Commands:
  new     Create a new feature branch with a conventional commit message
  finish  Commit changes and run a post-commit command
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## Developing
Use [bacon](https://dystroy.org/bacon/)

To install locally use cargo:
```bash
cargo install --path .
```
# vault-tasks

`vault-tasks` is a TUI Markdown task manager.

It will parse any Markdown file or vault and display the tasks it contains.

![Demo](./examples/vault-tasks.gif)

## Why

I made this tool because I wanted to integrate my task system directly inside my Second Brain.

Markdown tasks are very easy to integrate with knowledge and projects.

I also spend most of my writing time in the terminal (Helix) and do not rely on heavy extrernal software.

## Features

- Task Parser (see [Usage](https://github.com/louis-thevenet/vault-tasks/tree/main?tab=readme-ov-file#usage))
  - Subtasks
  - Fixed and relative dates
  - special _today_ tag and regular tags
  - descriptions
  - priority
- Navigate vault
- Search through tasks
- Open in default editor

## Planned Features

- A Pomodoro (or flowtime) tab
- A Timeline tab with a calendar and a chronological view. (I'd also like to be able to import calendar files)
- Manipulation of tasks (mark done, delete, ...) without the need to open editor
- Keybindings menu

## Installation

### Nix

It will be added to nixpkgs when I have time to make a nix module. For now, add the repo to your inputs:

```nix
vault-tasks = {
  url = "github:louis-thevenet/vault-tasks";
  inputs.nixpkgs.follows = "nixpkgs";
};
```

And use the package in your configuration: `inputs.vault-tasks.packages.${pkgs.system}.default`

### Build From Source

```
git clone https://github.com/louis-thevenet/vault-tasks.git
cd vault-tasks
cargo build --release
```

## Usage

See `vault-tasks --help` for basic usage.

### Writing tasks

```md
<!-- An example task in your markdown file -->

- [ ] An example task #tag tomorrow p1
      A description
      of this task
  - [x] A subtask today @today
  - [ ] Another subtask 10/23 @today
```

| Token                                      | Meaning                                                           |
| ------------------------------------------ | ----------------------------------------------------------------- |
| `- [ ]` (`- [X]`)                          | declares a task and sets its state                                |
| `p1` (`p10`, ...)                          | sets the priority                                                 |
| `#tag`                                     | is a tag, a task can have zero or more tags                       |
| `@today` (`@tod`, `@t`)                    | is a special tag that will mark the task as part of today's tasks |
| `23/10` (`2024/23/10`)                     | sets the due date with a literal date                             |
| `today` (`tdy`)                            | sets the due date to today                                        |
| `tomorrow` (`tmr`)                         | sets the due date to tomorrow                                     |
| a day of the week (`monday` or `mon`, etc) | sets the due date to the next occurence of that day               |
| `3d` (`3m, 3w, 3y`, ...)                   | means "in 3 days" and will set the due date accordingly           |

- `@today` allows you mark a task for today while keeping a due date. It will show up with a ☀️ in `vault-tasks`.

- Relative dates are always replaced by literal dates once `vault-tasks` is run. Thanks to this, `vault-tasks` does not store any data except its config file.

- Other tokens will be part of the title of that task

- Descriptions and subtasks are declared using indents (see config file)

This is what you will see in the preview of this `README.md` in `vault-tasks`:

![](./examples/readme_example_vault_tasks_explorer_screenshot.png)

Or when filtering for `@today` tasks:
![](./examples/readme_example_vault_tasks_filter_screenshot.png)

### Navigating `vault-tasks`

A keybindings menu should come with v0.4.0.

## Configuration

The [`config.toml`](./.config/config.toml) contains the default configuration which can be generated using `vault-tasks generate-config`.

In `$HOME/.config/vault-tasks/config.toml`, you can override the default settings, keybindings and colorscheme.

In particular, you can set a default vault path.

## Contributing

Feel free to submit issues or pull requests. Contributions are welcome!

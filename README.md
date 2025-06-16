# Vault-tasks

`vault-tasks` is a TUI Markdown task manager.

It will parse any Markdown file or vault and display the tasks it contains.

## Demo using `./test-vault`

```
./test-vault
├── dir
│   ├── subdir
│   │   ├── test_1.md
│   │   ├── test_2.md
│   │   └── test_3.md
│   └── test_0.md
├── example_physics_class.md
├── example_vault-tasks_project.md
└── test.md
```

![Demo explorer](./examples/demo_explorer.gif)
![Demo calendar](./examples/demo_calendar_view.png)

## Why

I made this tool because I wanted to integrate my task system directly inside my Second Brain.

Markdown tasks are very easy to integrate with knowledge/projects and are source-control-friendly.

I wrote a [blog post](https://louis-thevenet.github.io/blog/pkms/2025/04/12/personal-knowledge-management-and-tasks.html) explaining my workflow and how I use vault-tasks if you're interested. (It's dated, I introduced new features since then (trackers))

I also spend most of my writing time in the terminal (Helix) and do not rely on heavy external software.

## Features

- Task Parser (see [Usage](https://github.com/louis-thevenet/vault-tasks/tree/main?tab=readme-ov-file#usage))
  - Subtasks
  - Supports relative dates
  - special _today_ tag and regular tags
  - descriptions, priority, completion percentage
- Trackers (or recurrent tasks)
- Navigate vault
- Edit tasks or open in default editor
- Search through tasks (sort and filter)
- Calendar view
- Time Management tab (Pomodoro & Flowtime)

## Planned Features

I plan on adding a new tab about Trackers where you'll be able to see nice statistics about your trackers. For now, you can see your trackers in the Explorer tab.

## Installation

### Cargo

```
cargo install vault-tasks
```

### Nix

You can get it from nixpkgs 25.05 or directly from this repo's flake:

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
  - [/] Another subtask 10/23 @today c50
    Partly done
  - [-] This one is canceled
```

| Token                                      | Meaning                                                           |
| ------------------------------------------ | ----------------------------------------------------------------- |
| `- [ ]` (`- [x]`, ...)                     | declares a task and sets its state                                |
| `p1` (`p10`, ...)                          | sets the priority                                                 |
| `c50` (`c99`, `c150`, ...)                 | Sets the completion percentage                                    |
| `#tag`                                     | is a tag, a task can have zero or more tags                       |
| `@today` (`@tod`, `@t`)                    | is a special tag that will mark the task as part of today's tasks |
| `23/10` (`2024/23/10`)                     | sets the due date with a literal date                             |
| `today` (`tdy`)                            | sets the due date to today                                        |
| `tomorrow` (`tmr`)                         | sets the due date to tomorrow                                     |
| a day of the week (`monday` or `mon`, etc) | sets the due date to the next occurence of that day               |
| `3d` (`3m, 3w, 3y`, ...)                   | means "in 3 days" and will set the due date accordingly           |

- Task states are **Done** (`x`), **To-Do** (` `), **Incomplete** (`/`) and **Canceled** (`-`)

- `@today` allows you mark a task for today while keeping a due date. It will show up with a ☀️ in `vault-tasks`.

- Relative dates are always replaced by literal dates once `vault-tasks` is run. Thanks to this, `vault-tasks` does not store any data except its config file.

- Other tokens will be part of the title of that task

- Descriptions and subtasks are declared using indents (see configuration)

This is what you will see in the preview of this `README.md` in `vault-tasks`:

![](./examples/demo_readme_explorer.png)

<!-- Or when filtering for `@today` tasks: -->
<!-- ![](./examples/demo_filter.png) -->

### Writing Trackers (or recurrent tasks)

```md
<!-- An example tracker in your markdown file -->

Tracker: Arts Enjoyment (2025-06-08)

| Every day  | inspiration | medium   | duration | cost | social | energy | notes                      |
| ---------- | ----------- | -------- | -------- | ---- | ------ | ------ | -------------------------- |
| 2025-06-08 | 8           | painting | 2.5h     | 15   | [ ]    | high   | watercolor landscape       |
| 2025-06-09 | 6           | music    | 45min    | 0    | [x]    | medium | street musician downtown   |
| 2025-06-10 | 9           | theater  | 3h       | 45   | [x]    | medium | incredible Shakespeare     |
| 2025-06-11 | 7           | digital  | 1h       | 12   | [ ]    | low    | new photography exhibition |
| 2025-06-12 | 5           | craft    | 30min    | 8    | [ ]    | high   | pottery wheel attempt      |
| 2025-06-13 | 8           | film     | 2h       | 10   | [x]    | low    | foreign cinema festival    |
| 2025-06-14 | 9           | dance    | 1.5h     | 20   | [x]    | high   | salsa class breakthrough!  |
| 2025-06-15 |             |          |          |      |        |        |                            |
| 2025-06-16 |             |          |          |      |        |        |                            |
| 2025-06-17 |             |          |          |      |        |        |                            |
```

You start by writing `Tracker:` followed by your tracker's name. You add a starting date such as an ISO date or a relative date.

Then you can start vault-tasks, so it creates blank entries for you. By default, it will fill the entries until the current date + 3 blank entries. This is also useful to reformat your table if you don't have a Markdown formatter.

Here is a basic template:

```md
Tracker: Name (today)

| daily | notes |
```

Frequencies can be:

- `Every <hour|day|week|month|year>`
- `hourly`, `daily`, `weekly`, `monthly`, `yearly`

Data types are:

- Boolean type: `[ ]` or `[x]`
- Score type: any integer
- Note type: any text
- Blank

Be careful not to change type within a column, vault-tasks will replace the wrong type with a blank entry.

### Default Key Map

Check the key map within the app with `?`

#### General

| Key         | Alternate Key | Action                                    |
| ----------- | ------------- | ----------------------------------------- |
| `shift-h`   | `shift-←`     | Previous tab                              |
| `shift-l`   | `shift-→`     | Next tab                                  |
| `ctrl-k`    | `ctrl-↓`      | Scroll up                                 |
| `ctrl-j`    | `ctrl-↑`      | Scroll down                               |
| `page_down` |               | Scroll one page down                      |
| `page_up`   |               | Scroll one page up                        |
| `q`         | `ctrl-c`      | Quit the application                      |
| `?`         |               | Open keybindings menu for the current tab |

#### Explorer Tab

##### Navigation

| Key | Alternate Key     | Action              |
| --- | ----------------- | ------------------- |
| `k` | `↑`, `shift-tab`  | Previous entry      |
| `j` | `↓`, `tab`        | Next entry          |
| `h` | `←`, `back_space` | Leave current entry |
| `l` | `→`,`enter`       | Enter current entry |

##### Commands

| Key | Action                                         |
| --- | ---------------------------------------------- |
| `s` | Focus search bar (`enter` or `esc` to unfocus) |
| `o` | Open selection in default editor               |
| `e` | Quickly edit selection                         |
| `r` | Reload vault                                   |
| `t` | Mark task **To-Do**                            |
| `d` | Mark task **Done**                             |
| `i` | Mark task **Incomplete**                       |
| `c` | Mark task **Canceled**                         |

![](./examples/demo_explorer.gif)

#### Filter Tab

##### Commands

| Key       | Action                   |
| --------- | ------------------------ |
| `enter`   | Focus/Unfocus search bar |
| `Shift-s` | Change sorting mode      |

![](./examples/demo_filter_view.png)

#### Calendar Tab

##### Navigation

| Key       | Alternate Key | Action   |
| --------- | ------------- | -------- |
| `h`       | `←`           | +1 day   |
| `l`       | `→`           | -1 day   |
| `j`       | `↓`           | +7 days  |
| `k`       | `↑`           | -7 days  |
| `Shift-j` | `Shift-↓`     | +1 month |
| `Shift-k` | `Shift-↑`     | -1 month |
| `n`       |               | +1 year  |
| `Shift-n` |               | -1 year  |

##### Commands

| Key | Action     |
| --- | ---------- |
| `t` | Goto Today |

![](./examples/demo_calendar_view.png)

#### Time Management Tab

##### Navigation

| Key | Alternate Key | Action           |
| --- | ------------- | ---------------- |
| `k` | `↑`           | Previous setting |
| `j` | `↓`           | Next setting     |

##### Commands

| Key         | Action                             |
| ----------- | ---------------------------------- |
| `space`     | Next segment (skip current)        |
| `p`         | Pause timer                        |
| `e`         | Edit selected setting              |
| `shift-tab` | Previous time management technique |
| `tab`       | Next time management technique     |

![](./examples/demo_time_view.png)

### Modes

You can start already focused on a tab by using one of the CLI subcommands:

```bash
vault-tasks explorer # is the default
# Or
vault-tasks filter
vault-tasks time
vault-tasks calendar
```

```

```

You can also output the content of a vault in standard output using

```bash
vault-tasks stdout
```

Example output:

```
vault-tasks -v ./README.md stdout
./README.md
‾‾‾‾‾‾‾‾‾‾‾
	README.md
	‾‾‾‾‾‾‾‾‾
		Vault-tasks
		‾‾‾‾‾‾‾‾‾‾‾
			Usage
			‾‾‾‾‾
				Writing tasks
				‾‾‾‾‾‾‾‾‾‾‾‾‾
					❌ An example task
					📅 2025-03-31 (tomorrow) ❗1
					#tag
					A description
					of this task

						✅ A subtask
						☀️ 📅 2025-03-30 (today)


						⏳ Another subtask
						☀️ 📅 2025-10-23 (in 7 months) [🟩🟩⬜️⬜️⬜️ 50%]
						Partly done


						🚫 This one is canceled
```

## Configuration

The [`config.toml`](./.config/config.toml) contains the default configuration which can be generated using `vault-tasks generate-config`.

In `$HOME/.config/vault-tasks/config.toml`, you can override the default settings, keybindings and colorscheme.

In particular, you can set a default vault path.

## Contributing

Feel free to submit issues or pull requests. Contributions are welcome!

```

```

```

```

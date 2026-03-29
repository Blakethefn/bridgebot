# bridgebot

A background daemon that watches your [Obsidian](https://obsidian.md) vault and local git repos, reacting to changes with configurable rules for auto-linking, staleness alerts, and vault maintenance.

## What it does

- **Watches your vault** for file changes (creates, edits, deletes) via inotify
- **Watches git repos** for commits, branch switches, and ref changes
- **Runs configurable rules** that trigger actions based on events
- **Sends desktop notifications** when tasks go stale or links break
- **Auto-backlinks** new output notes from their project hubs
- **Updates frontmatter** dates when git activity is detected
- **Checks wikilinks** for broken references on save or on a schedule
- **Runs shell commands** as custom actions

## Install

```bash
cargo install --path .
```

Or build from source:

```bash
git clone https://github.com/Blakethefn/bridgebot.git
cd bridgebot
cargo build --release
```

## Quick start

Generate a default config:

```bash
bridgebot init
```

Edit `bridgebot.toml` to point at your vault:

```toml
[daemon]
vault = "/path/to/your/obsidian-vault"
projects_dir = "/path/to/your/projects"  # optional
interval = 300
```

Validate it:

```bash
bridgebot check
```

Start the daemon:

```bash
bridgebot start
```

Run all rules once (good for cron or testing):

```bash
bridgebot run
```

## Configuration

bridgebot looks for config in this order:

1. `--config <path>` (explicit)
2. `./bridgebot.toml` (current directory)
3. `~/.config/bridgebot/bridgebot.toml` (XDG config)
4. `~/.bridgebot.toml` (home directory)

### Rules

Each rule has a trigger and an action:

```toml
[[rule]]
name = "stale-task-alert"
trigger = "task.active AND task.no_update_days > 7"
action = "notify"
message = "Task '{task.name}' has been active for {task.no_update_days} days"
```

### Triggers

| Trigger | Fires when |
|---------|-----------|
| `task.active AND task.no_update_days > N` | Periodic check finds stale tasks |
| `output.created` | A new note with `type: output` appears |
| `vault.file_saved` | Any markdown file is created or modified |
| `vault.broken_link` | Periodic check (use with `check-links` action) |
| `git.commit AND task.matches_branch` | A git ref changes in a watched project |

### Actions

| Action | Description | Required fields |
|--------|-------------|----------------|
| `notify` | Desktop notification | `message` |
| `check-links` | Check for broken wikilinks | `scope` (optional: `"changed-file"`) |
| `link-from-hub` | Backlink new notes from their project hub | — |
| `update-frontmatter` | Update a frontmatter field | `field` (default: `"updated"`) |
| `shell` | Run a shell command | `command` |

### Template variables

Available in `message` and `command` fields:

| Variable | Value |
|----------|-------|
| `{task.name}` | Task note filename (in notify) |
| `{task.no_update_days}` | Days since last update (in notify) |
| `{vault}` | Vault root path (in shell) |
| `{file}` | Full path of the triggering file (in shell) |
| `{file.name}` | Filename without extension (in shell) |

### Vault expectations

bridgebot expects your vault notes to use YAML frontmatter:

```yaml
---
type: task        # or: output, project
status: active    # or: done, blocked, paused, archived
updated: 2026-03-29
project: my-project
---
```

It looks for project hubs at `01-projects/<project>.md` or `01-projects/<project>/<project>.md`.

## Example config

```toml
[daemon]
vault = "~/Documents/my-vault"
projects_dir = "~/Projects"
interval = 300
ignore = [".obsidian", ".git", ".trash"]

[[rule]]
name = "stale-task-alert"
trigger = "task.active AND task.no_update_days > 7"
action = "notify"
message = "Task '{task.name}' has been active for {task.no_update_days} days"

[[rule]]
name = "broken-link-warning"
trigger = "vault.file_saved"
action = "check-links"
scope = "changed-file"

[[rule]]
name = "auto-backlink-output"
trigger = "output.created"
action = "link-from-hub"

[[rule]]
name = "update-task-on-commit"
trigger = "git.commit AND task.matches_branch"
action = "update-frontmatter"
field = "updated"

[[rule]]
name = "custom-script"
trigger = "vault.file_saved"
action = "shell"
command = "echo 'File changed: {file.name}' >> /tmp/bridgebot.log"
```

## License

MIT

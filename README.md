# Jacu Fast

Jacu Fast is a workflow for coding agents. It shortens the path to a change that is actually finished: the behavior you asked for is present, the checks that matter have run, and the work is in the tree you meant to ship.

A faster "done" message is not the goal. A missing requirement, a skipped test, or work left in another worktree still counts as unfinished.

Version 0.0.2 is the skill and the specification. The Rust command, `jacu`, is described in [the plan](docs/IMPLEMENTATION_PLAN.md) and is not in this release. Installing the skill does not install a binary, and it does not make a test suite faster on its own. Nothing in this release requires macOS 14 or any other specific operating-system version.

## Install

These are two different installs. The marketplace plugin is what Claude Code loads. The skills installer only copies the skill file into an agent's skill directory. Neither one installs a `jacu` binary.

### Claude Code

In Desktop, paste the HTTPS repository URL. The `owner/repo` shorthand can select SSH, and a non-interactive Desktop process may stall on that choice.

```text
https://github.com/jacu-dev/jacu-fast.git
```

From a terminal where `claude` is already installed:

```text
claude plugin marketplace add https://github.com/jacu-dev/jacu-fast.git
claude plugin install jacu-fast@jacu-fast
```

Start a new session. The plugin command is `/jacu-fast:jacu-fast` followed by the task. There is no `/jacu-fast:run` command in this release.

### Skills installer

`npx --yes` skips npm's own install prompt. `-s jacu-fast` selects this skill. `-a` names the agents that should receive it. `--all` means every skill for every agent the installer supports, not only the agents already present on the machine.

```bash
npx --yes skills add jacu-dev/jacu-fast -g -y --skill jacu-fast -a claude-code
```

A skill installed this way, and not through the plugin above, is invoked as `/jacu-fast`. Add further agent names to `-a` only when that agent should receive the skill. Node is used only for the copy. Jacu does not run as a Node service. There is no OpenCode npm plugin and no MCP server.

Update the copied skill:

```bash
npx --yes skills update jacu-fast -g -y
```

Remove it:

```bash
npx --yes skills remove jacu-fast -g -y
```

The installed skill uses the project's own checks. It must not search for or run `jacu`, and it must not invent timings or a passing result.

## What the agent does

The skill keeps the current session. It does not start another agent, switch models, or ask you to approve routine steps.

1. Look for an implementation that already exists before writing it again.
2. Keep a short list of the outcomes in the original request.
3. Run the next useful check, not the full suite after every edit.
4. Reuse a passing result only when the inputs that result depends on are unchanged.
5. Call the work complete only when those outcomes are present, evidenced, and in the delivery tree.

A real blocker ends as incomplete, with the missing piece named. There is no Jacu approval prompt.

The command contract for a later executable is in [skills/jacu-fast/references/runtime.md](skills/jacu-fast/references/runtime.md). Those commands are not in version 0.0.2.

## What is in this repository

| Path | Contents |
| --- | --- |
| `skills/jacu-fast/` | The skill the installer copies |
| `docs/IMPLEMENTATION_PLAN.md` | The engineering specification |
| `docs/acceptance-cases.json` | The checklist the CLI must pass before a release can claim those behaviors |
| `examples/policy.json` | A synthetic example of a future check policy |
| `plugin.json` | Portable plugin identity for hosts that read it |
| `.claude-plugin/marketplace.json` | Claude Code marketplace catalog |
| `.claude-plugin/plugin.json` | Claude Code plugin manifest for the skill |
| `LICENSE` | MIT license |

`examples/policy.json` is an illustration. It is not a policy to drop into another repository.

## License

[MIT](LICENSE). Copyright (c) 2026 Jacu Fast contributors.

You may use, copy, modify, merge, publish, and distribute this project, including in commercial work and inside a larger product. The only condition is that copies and substantial portions keep the copyright notice and this license text. The project is provided without warranty, and the authors are not liable for damages that come from using it.

Those are the MIT terms as published by [Choose a License](https://choosealicense.com/licenses/mit/). GitHub detects the license from a standard `LICENSE` file in the repository root ([licensing a repository](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository)). A public repository without that file is visible and forkable, and it does not grant others the right to reuse the code.

Third-party code added later keeps its own notices.

## Contributing

Issues and pull requests are welcome. Write public text in English. Keep examples synthetic. Do not put secrets, customer data, or material from a private repository into an issue or a commit.

The behaviors in `docs/acceptance-cases.json` are the bar for the CLI. A prose change does not mark one of them passed.

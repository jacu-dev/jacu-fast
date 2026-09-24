# Jacu Fast

Jacu Fast is a workflow for coding agents. It shortens the path to a change that is actually finished: the behavior you asked for is present, the checks that matter have run, and the work is in the tree you meant to ship.

A faster "done" message is not the goal. A missing requirement, a skipped test, or work left in another worktree still counts as unfinished.

The public repository is the skill and the specification. The Rust command, `jacu`, is described in [the plan](docs/IMPLEMENTATION_PLAN.md) and is not released yet. Installing the skill does not install a binary, and it does not make a test suite faster on its own.

## Install

This is the install that matches the repository today. One command copies the skill into the coding agents already on your machine. The installer knows Grok, OpenCode, Cursor, Claude Code, Codex, and many others.

```bash
npx skills add jacu-dev/jacu-fast -g --all
```

To install it only for the agents you use:

```bash
npx skills add jacu-dev/jacu-fast -g -y -a grok opencode cursor claude-code codex
```

`-g` puts the skill in your user directory, so it is available in every project. `--all` skips the prompts and selects every skill and every detected agent. Node is used only for that copy. Jacu does not run as a Node service.

Claude Code can also add this repository as a marketplace. The catalog is `.claude-plugin/marketplace.json`, and the plugin source is this repository. It installs the same skill. It does not install a `jacu` binary.

```text
/plugin marketplace add jacu-dev/jacu-fast
/plugin install jacu-fast@jacu-fast
```

Start a new agent session after installing. Invoke the skill with `/jacu-fast` and the task. Claude Code namespaces plugin skills, so the same skill there is `/jacu-fast:jacu-fast`. OpenCode, Cursor, and Grok read the skill from their own skill directories. The skills installer writes the file into each of those directories. There is no OpenCode npm plugin and no MCP server.

Until a `jacu` release exists, the skill has no executable to call. The agent should say that and keep using the project's own checks. It should not invent timings or a passing result.

Update later with:

```bash
npx skills update jacu-fast -g
```

Remove it with:

```bash
npx skills remove jacu-fast -g -y
```

## What the agent does

The skill keeps the current session. It does not start another agent, switch models, or ask you to approve routine steps.

1. Look for an implementation that already exists before writing it again.
2. Keep a short list of the outcomes in the original request.
3. Run the next useful check, not the full suite after every edit.
4. Reuse a passing result only when the inputs that result depends on are unchanged.
5. Call the work complete only when those outcomes are present, evidenced, and in the delivery tree.

A real blocker ends as incomplete, with the missing piece named. There is no Jacu approval prompt.

The command contract the future CLI will follow is in [skills/jacu-fast/references/runtime.md](skills/jacu-fast/references/runtime.md). Those commands are proposed. They are not a tool you can run from this commit.

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

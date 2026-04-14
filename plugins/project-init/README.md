# project-init

Multi-language project setup plugin. Sets up code conventions, dependencies, `CLAUDE.md`, and installs a language-specific convention skill into the target project.

## Installation

Install via the marketplace:

```
/plugin marketplace add 0pg/cc-marketplace
/plugin install project-init@jhk-plugins
```

## Usage

```
/project-init [--lang rust] [--name <name>] [--type cli,backend,frontend] [--db toasty|none|<custom>]
```

### Interactive mode

Run `/project-init` with no arguments; the command will prompt for language, project name, type, and database.

### Direct mode

Provide arguments to skip the prompts:

```
/project-init --lang rust --name my-app --type cli
```

## What it does

1. **Language selection** — interactive or via `--lang`
2. **Argument parsing** — interactive vs direct mode
3. **VCS + project init** — idempotent (`git init`, `cargo init`, etc.)
4. **Formatter config** — language-specific (e.g. `rustfmt.toml`)
5. **Lints merge** — upsert, preserving existing entries
6. **Dependencies** — adds common + type-specific deps
7. **CLAUDE.md** — generates from the language template
8. **Convention skill** — installs `{lang}-convention` into `.claude/skills/` of the target project
9. **Superpowers plugin** — guides the user through installation
10. **Summary** — prints the applied changes

## Supported languages

- **Rust** — `rustfmt.toml`, `cargo-lints.toml`, common/CLI/backend/frontend dependency sets, `claude-md-template.md`, convention skill

## Extending to a new language

1. Create `commands/references/{lang}/`
2. Add formatter config, lints, dependency TOMLs, CLAUDE.md template, and `convention/SKILL.md`
3. Add a `### {Lang} (LANG == {lang})` section to each phase in `commands/project-init.md`

## License

MIT

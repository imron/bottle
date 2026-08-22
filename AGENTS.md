# First-principles operating rules

- Strip each task to its base facts. If a claim cannot survive "why is this
  true," do not build on it.
- Challenge the received answer. If "this is how everyone does it" is the only
  reason, treat it as a constraint to question, not a rule to follow.
- Rebuild from the base facts with the fewest moves that reach the outcome.
  Cut steps that add ceremony but not value.
- Lean on the type system to make it impossible to represent invalid state

## Commits

One commit is one complete, revertible unit of work. Do not mix unrelated
changes. If a commit cannot be described in one subject line, split it.
See https://www.aleksandrhovhannisyan.com/blog/atomic-git-commits/

Write messages as in https://cbea.ms/git-commit/ : imperative subject around
50 characters, no trailing period, blank line, body wrapped at 72 that
explains why.

A commit does not land until all of these pass on that commit's tree:

- `cargo fmt --check`
- `cargo check --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- `/code-review` on the commit's diff, with no bugs

Do not skip `/code-review`. Fix bugs it finds, then review again.

Do not silence clippy or rustc lints (`#[allow(...)]`, `#![allow(...)]`,
`expect`, `--allow`, `#![allow(clippy::...)]`) without explicit human
approval for that lint at that site. Restructure the code instead.

### Pre-commit hook

`.githooks/pre-commit` runs fmt, `cargo check`, and clippy so those
cannot be forgotten.
Point git at it once per clone:

```
git config core.hooksPath .githooks
```

`/code-review` is not in the hook. A hook can run compilers; it cannot run
an agent skill. The committer (human or agent) still has to run
`/code-review` on the diff and fix bugs before `git commit`. Do not use
`--no-verify` to skip the hook.

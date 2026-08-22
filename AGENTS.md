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

A commit does not land until `cargo fmt --check` is clean, `cargo clippy
--all-targets -- -D warnings` is clean, and `/review` (the code-review skill)
reports no bugs.

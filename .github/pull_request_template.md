## What

<!-- 1-3 sentences. Refer to issue: Closes #123 -->

## Why

<!-- The motivation. Skip if obvious from "What". -->

## How

<!-- Notable implementation choices. -->

## Checklist

- [ ] Conventional commit title (`feat|fix|chore|docs|refactor|test|ci|perf`)
- [ ] No mention of AI / Claude / Anthropic / Copilot anywhere
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test --workspace` passes
- [ ] `pnpm --dir web lint && pnpm --dir web typecheck && pnpm --dir web test` passes (if web touched)
- [ ] Mobile-first respected (designed at 360 px first)
- [ ] No new `unsafe`, no `unwrap()` in non-test code
- [ ] No new secret in code/commits/`.env` versionned
- [ ] Contract changes (if any) propagated to `contracts/` AND noted here
- [ ] Docs updated (`README.md`, `docs/`, `CLAUDE.md` if rules changed)

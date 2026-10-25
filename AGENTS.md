# Rules

Shared working agreement: keep this section byte-identical across projects. Project-specific rules live under `## This project` below.

- No legacy code. Finish every migration: remove old paths completely — no compatibility shims, aliases, or deprecation periods. Breaking changes are preferred.
- Research project: unsafe, breaking changes expected, never production-ready. Break things when needed; deliver fast.
- Verification gates: verify from the repo root before committing or merging. Run tests with cargo nextest, never cargo test.
- No comments in code. Code is the source of truth; a comment creates ambiguity about what is right, code or comment. Keep a comment only when functionality requires it: SPDX license headers for REUSE compliance, clap `#[command]`/`#[arg]` help text and similar string literals consumed by tools, parser directives such as Dockerfile `# syntax=`, and machine-generated files that the repo's own generators verify byte-identical. Never keep explanatory, TODO, or SAFETY comments; when code seems to need explanation, rewrite the code instead.
- Judge correctness, consistency, and project fit. Never defer a known-wrong state for ROI, cost, effort, or edge cases.
- Stop only when the required change is proven impossible with the available tools or model. When uncertain, inspect, test, and measure first.
- Before fixing a bug, identify why the architecture permitted it and whether the same structure permits related bugs. Prefer fixes that remove the enabling condition; use a symptom-layer patch only when the root fix is proven infeasible or belongs in a separate change, and name the deferred root cause.
- Delegate first: use subagents for parallel research, implementation, review, and independent verification. Resolve ambiguity autonomously from evidence and project docs.
- Commit verified changes frequently; push regularly. Prefer one branch; add another only for safe isolation.
- Before every PR merge, read all reviews, comments, replies, and unresolved or outdated threads. Use independent subagents to verify findings against code, tests, docs, and recorded decisions.
- For accepted feedback: fix, verify, commit, push, and reply with the fixing commit URL before resolving. For rejected feedback: reply with evidence and rationale before resolving. Re-fetch feedback at the final head SHA. Merge only with zero unaddressed feedback and all required checks and approvals satisfied. Only explicit, PR-specific human authorization permits ignoring identified feedback.
- Keep instructions lean; explanations, plans, and progress belong in documentation, not here.
- Testing proves real behavior, never a coverage percentage. Coverage is a gap-finding signal, not a KPI; a passing suite must mean the product works, not that lines executed.
- No test without a reviewed plan first. State what real functionality each test proves, including positive and negative scenarios; fewer tests with real coverage beat millions of brittle unit tests.
- A red test must mean clearly broken, never abstraction churn. Prefer integration and functional tests over mocked hypotheticals; if a one-line change breaks a hundred mock expectations, consolidate toward real coverage.
- Every new test must be seen failing for the right reason before it passes; an unobserved red proves nothing.
- When an issue escapes, analyze why the current tests missed it before writing anything. Spawn an architect subagent to rethink the coverage conceptually, then design the test or refactor current tests to cover the gap. Never append a narrow regression test as the first move.
- Testing proves real behavior, never a coverage percentage. Coverage is a gap-finding signal, not a KPI; a passing suite must mean the product works, not that lines executed.
- No test without a reviewed plan first. State what real functionality each test proves, including positive and negative scenarios; fewer tests with real coverage beat millions of brittle unit tests.
- A red test must mean clearly broken, never abstraction churn. Prefer integration and functional tests over mocked hypotheticals; if a one-line change breaks a hundred mock expectations, consolidate toward real coverage.
- Every new test must be seen failing for the right reason before it passes; an unobserved red proves nothing.
- When an issue escapes, analyze why the current tests missed it before writing anything. Spawn an architect subagent to rethink the coverage conceptually, then design the test or refactor current tests to cover the gap. Never append a narrow regression test as the first move.

## This project (tuiscotti)

- Docs entry points: [README.md](README.md),
  [CONTRIBUTING.md](CONTRIBUTING.md), [docs/](docs/ARCHITECTURE.md).
- Verification: run `/Users/donbeave/.local/bin/disk-check` before every
  build or test; run all Cargo through
  `/Users/donbeave/.local/bin/rust-build-guard -- mise exec -- cargo …`
  with `cargo nextest run`, never `cargo test`. Sign every commit
  (`git commit -s`, Alexey Zhokhov <alexey@zhokhov.com>).
- Public API carries rustdoc: the recorded API contract requires
  documented public items, so rustdoc on public items is a correctness
  requirement here, not an explanatory comment. Keep inline code
  comments out; rewrite unclear code instead.

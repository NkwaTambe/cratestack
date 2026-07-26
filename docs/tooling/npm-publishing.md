# npm + crates.io publishing setup

`.github/workflows/release-cli.yml` publishes everything this repo ships on every `vX.Y.Z` tag
push (never on a manual `workflow_dispatch` — none of these publishes can be deleted and retried
like a GitHub Release, so a throwaway test tag must never reach a registry):

- **Every workspace crate** (`publish-crates` job) — topo-sorted `cargo publish` via
  `just release-publish real`, same recipe a human would run locally.
- **`@cratestack/cli`** (`publish-npm` job, `packages/cratestack-cli-npm/`) — fetches the
  prebuilt binary from the matching GitHub Release at install time.
- **`@cratestack/api`** (`publish-npm-api` job, `packages/cratestack-api/`) — hand-written, ships
  its own compiled `dist/` in the tarball.

All three jobs soft-skip (log a warning, exit 0, without failing the rest of the release) when
their respective secret isn't set — so the release still succeeds even before every secret below
is configured. This tag is normally produced by the **"Prepare Release"** →
**"Cut Release Tag"** pipeline described in [`RELEASE.md`](../../RELEASE.md), not pushed by hand.

## npm one-time setup (needs `@cratestack` npm org access)

1. On npmjs.com, sign in as a member of the `@cratestack` org with publish rights.
2. Create a new **Automation** access token (Settings → Access Tokens → Generate New Token →
   Granular Access Token or Automation, scoped to `@cratestack/cli` and `@cratestack/api`). An
   Automation token is required here — a token requiring 2FA-on-publish won't work in CI.
3. In the GitHub repo (`cratestack/cratestack`) → Settings → Secrets and variables → Actions, add
   a new repository secret named `NPM_TOKEN` with that token's value.

Once the secret exists, the next tag push publishes both npm packages — no other change needed.

**How to tell you got the token type wrong:** if `NPM_TOKEN` is a regular (non-Automation) token
from an account with 2FA-on-publish enabled, both `publish-npm` and `publish-npm-api` fail with
`npm error code EOTP` / `npm error This operation requires a one-time password from your
authenticator.` This is the specific, recognizable symptom of this exact misconfiguration — npm
has no way to satisfy an OTP challenge from unattended CI, so the fix is always "rotate to a real
Automation token," never "retry." Confirmed the hard way on `v0.4.15` (both npm publishes failed
with `EOTP`); rotating `NPM_TOKEN` to an Automation token and cutting `v0.4.16` instead confirmed
the fix — see [`RELEASE.md`'s Troubleshooting section](../../RELEASE.md#npm-publish-fails-with-eotp--this-operation-requires-a-one-time-password)
and the live verification commands there.

## crates.io one-time setup

1. On crates.io, sign in as an account with publish rights on every `cratestack-*` crate.
2. Create a new API token (Account Settings → API Tokens → New Token), scoped at minimum to
   `publish-new` and `publish-update`.
3. Add it as a repo secret named `CARGO_REGISTRY_TOKEN` (same Settings → Secrets and variables →
   Actions page as `NPM_TOKEN`) — `cargo publish` reads this env var automatically, no extra
   config needed.

Once the secret exists, the next tag push publishes every workspace crate via `publish-crates`
(idempotent — already-published versions are skipped, so a re-run after a partial failure, e.g. a
transient crates.io index lag, is safe).

## Release-tag one-time setup (`RELEASE_PAT`)

`.github/workflows/cut-release-tag.yml` creates and pushes the `vX.Y.Z` tag once a "Prepare
Release" bump PR merges — but GitHub does not fire other workflows' triggers for a push made with
the default `GITHUB_TOKEN` (anti-recursion protection), so without this secret the tag gets created
correctly but **`release-cli.yml` never runs and nothing actually gets published** — confirmed the
hard way on `v0.4.14`'s first real release through this pipeline. `cut-release-tag.yml` logs a
loud `::warning::` when this secret is missing, precisely so that failure mode isn't silent again.

1. On GitHub, create a **personal access token** with `contents: write` permission on
   `cratestack/cratestack` (a fine-grained PAT scoped to just this repo is preferred over a classic
   PAT with the broader `repo` scope, but either works).
2. Add it as a repo secret named `RELEASE_PAT` (same Settings → Secrets and variables → Actions
   page as `NPM_TOKEN`/`CARGO_REGISTRY_TOKEN`).

Once the secret exists, the next "Prepare Release" bump PR that merges will have its auto-created
tag genuinely trigger `release-cli.yml` — no manual `gh workflow run`/tag recreation needed.
Confirmed working on `v0.4.15` and `v0.4.16`: both releases' `release-cli.yml` runs show
`event: "push"` (not `workflow_dispatch`), i.e. the tag push genuinely cascaded.

## Known limitation: this repo cannot fully self-serve PR creation

Separately from the three secrets above, "Prepare Release" (`mode: real`) itself cannot currently
open its own bump PR — the `gh pr create` call in its "Open release PR" step fails with `GitHub
Actions is not permitted to create or approve pull requests`. This is an org-level GitHub setting
(Settings → Actions → General → Workflow permissions → "Allow GitHub Actions to create and approve
pull requests" is off), confirmed to also reject being flipped via the API (409: "The organization
does not allow GitHub Actions to create or approve pull requests"). No repo secret fixes this — it
is a standing, unresolved limitation with a manual workaround (the bump commit and branch push
still succeed; a human opens the PR by hand for the pushed branch). See
[`RELEASE.md`'s Troubleshooting section](../../RELEASE.md#pr-creation-fails-github-actions-is-not-permitted-to-create-or-approve-pull-requests)
for the exact recovery commands.

## Provenance

Both publish steps pass `npm publish --provenance`, which attaches a
[Sigstore-signed provenance attestation](https://docs.npmjs.com/generating-provenance-statements)
linking the published tarball back to this exact GitHub Actions run and commit. This needs:

- **A public repository** — provenance publishing is rejected for private repos. Already satisfied.
- **`id-token: write` permission** — set at the job level on `publish-npm` and `publish-npm-api`
  (not workflow-wide, since the other jobs in this file don't need it).
- **npm >= 9.5.0** — whatever ships with the pinned `node-version: 20` in `actions/setup-node`
  already satisfies this.

No additional secret or npmjs.com configuration is needed for provenance beyond the `NPM_TOKEN`
above — it's purely a CI-side capability enabled by the permission and the flag.

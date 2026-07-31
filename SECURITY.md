# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| main    | ✅ |
| Other branches / tags | ❌ |

Only the `main` branch receives security patches. Previous releases have no active security maintenance.

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

Send the report by email to **hanserlodev@gmail.com** (or through whatever channel Hans specifies in the repository profile).

### What to Include in the Report

- A clear description of the vulnerability and its impact.
- Steps to reproduce it (or a PoC if possible).
- The affected version/commit.
- If you already have a suggested mitigation or patch, include it.

### What to Expect After Reporting

1. **Acknowledgement of receipt** within 72 hours.
2. **Evaluation and response** (is it valid?, severity, patch plan) within 7 days.
3. **Disclosure coordination**: we work with a reasonable embargo period before publishing the fix and the advisory.

If the vulnerability is critical, we prioritize a hotfix on `main` over anything else.

## Scope

This repository contains:

- `vor-core` / `vor-import` / `vor-sim` / `vor-render` / `vor-edit` / `vor-app` / `vor-cli` — the Voronia engine (Rust + wgpu).
- Azgaar map import tools (`.map`/JSON) and the `.vorn` format.

**Out of scope** (not considered vulnerabilities): third-party code already published on crates.io (report to the maintainer of the corresponding crate), and outdated dependencies with no demonstrated exploit.

## Best Practices (Reminder)

- Everything generative uses an explicit seed: **same seed + same parameters = same result**. If you find a generator without a seed, it is a bug.
- The renderer **never writes** to the World Data Model (`vor-render` is read-only). Any write from the renderer is an architecture violation and a candidate security/data-integrity bug.
- Secrets and keys are never committed. `.env` files do not exist in this repository.

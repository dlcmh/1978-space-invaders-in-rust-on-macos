# Security Policy

## Supported versions

| Version | Supported |
| --- | --- |
| `main` / 1.x | Yes |
| Older snapshots | No |

The project is a local-only fan game. There is no server component and no network
access by default.

## Reporting a vulnerability

Please **do not open a public GitHub issue** for a suspected security
vulnerability.

Report it privately through GitHub's private vulnerability reporting:

<https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos/security/advisories/new>

If private advisories are unavailable, contact the maintainer
[@dlcmh](https://github.com/dlcmh) through GitHub and request a private channel.
Do not include exploit details in a public issue.

In your report, include:

- the affected version or commit;
- macOS, Rust, and hardware details;
- a concise description of the issue and its impact;
- reproduction steps or a minimal proof of concept; and
- any suggested mitigation.

## Response expectations

This is a volunteer-maintained fan project. The maintainer will make a best effort
to acknowledge reports, assess severity, and coordinate a fix and disclosure. No
response-time or remediation SLA is offered, and no bug bounty is available.

Please allow reasonable time for a fix before public disclosure. The maintainer
will coordinate disclosure timing with the reporter.

## Scope

The intended attack surface is deliberately small:

- a native macOS application that runs locally;
- no network requests, telemetry, accounts, or remote configuration;
- no runtime asset downloads;
- a small dependency set limited to Apple-framework bindings; and
- local `NSUserDefaults` only for high score and mute state.

Reports are in scope when they describe a concrete security impact in this
repository, for example unsafe packaging, unexpected data access, a compromised
build path, a dependency/asset issue that enters the app, or a memory-safety issue
in the `unsafe` Objective-C/Metal boundary.

Out of scope includes:

- social engineering and physical access to an unlocked machine;
- vulnerabilities in macOS, Rust, the Rust toolchain, Metal, or the user's
  hardware that are not caused by this project;
- issues requiring a malicious local user to alter the app bundle or the user's
  preferences without a trust-boundary crossing;
- denial-of-service from manually editing local preferences or resizing the
  window to pathological sizes;
- trademark or licensing questions (see [GOVERNANCE.md](GOVERNANCE.md)); and
- reports against unofficial forks or modified distributions.

## Local data and privacy

The app stores only:

| Preference key | Data | Purpose |
| --- | --- | --- |
| `SpaceInvadersArcadeHighScore` | Integer | Local high score |
| `SpaceInvadersArcadeMuted` | Boolean | Sound preference |

There is no analytics, telemetry, crash uploader, advertising, or remote
persistence. The game does not transmit gameplay data. Users can remove both
preferences with:

```bash
defaults delete dev.dlcmh.space-invaders-rust-metal
```

## Build integrity

- Builds are produced from source with `cargo build`, `make app`, or `make dmg`.
- The app bundle is ad-hoc signed for local use; it is not Developer ID signed or
  notarized for general distribution.
- No prebuilt binary is committed to the repository.
- Dependencies are locked in `Cargo.lock`, and the release/check commands use
  `--locked`.
- Do not download or run third-party modified builds claiming to be official.
- The canonical source is
  <https://github.com/dlcmh/1978-space-invaders-in-rust-on-macos>.

## Unsafe code policy

`space-invaders-core` forbids unsafe code. The application crate contains unsafe
operations at the Objective-C and Metal boundary, each with a `SAFETY` comment
naming the framework contract or pointer invariant it relies on. Changes to those
blocks should be reviewed with particular care, especially buffer sizes, drawable
lifetimes, completion-handler lifetimes, and the audio callback's user-data
pointer.

## Dependency policy

The accepted dependencies are the `objc2` framework bindings and `block2`. Any
proposed dependency requires maintainer approval and a review of its security,
licensing, and maintenance impact. See [GOVERNANCE.md](GOVERNANCE.md).

## Legal and project context

See [GOVERNANCE.md](GOVERNANCE.md) for licensing, trademark, and project policy.

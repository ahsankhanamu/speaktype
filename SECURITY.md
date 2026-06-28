# Security Policy

## Supported versions

Security fixes are applied to the latest release on the `main` branch.

## Reporting a vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Report security issues privately by emailing the maintainers (see the GitHub profile contact on the repository) or via GitHub's private vulnerability reporting if enabled.

Include:

- Description of the issue and potential impact
- Steps to reproduce
- Affected component (widget, CLI, Python server)

We aim to acknowledge reports within a few days.

## Scope

SpeakType processes audio locally on your machine. By default, no data is sent to external services unless you explicitly configure an external API endpoint in the CLI.

For technical hardening notes on the Python server, see [docs/security.md](docs/security.md).

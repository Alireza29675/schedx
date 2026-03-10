# Security Policy

## Supported Versions

Until `1.0.0`, only the latest tagged release is supported for security fixes.

## Reporting a Vulnerability

Do not open a public issue for undisclosed security problems.

Report vulnerabilities through GitHub Security Advisories for this repository, or contact the maintainer privately if advisories are not yet enabled. Include:

- the affected version
- operating system and architecture
- reproduction steps
- impact assessment
- whether credentials, tokens, or arbitrary command execution are involved

## Security Notes

- `schedx` runs commands and agent binaries configured by the local user. Treat scheduled actions as trusted local code.
- webhook logs can contain remote server output. Do not schedule jobs against untrusted endpoints unless you are comfortable storing that output locally.
- direct release installers verify release checksums before installing.

# Security Policy

`seishin` is an early engine prototype. Security-sensitive areas are still small but important:

- C ABI / FFI pointer and ownership boundaries;
- asset path validation and root containment;
- unsafe renderer surface creation from platform window handles;
- panic containment across FFI.

## Supported Versions

No stable release line exists yet. Please report issues against `main`.

## Reporting a Vulnerability

Please report vulnerabilities through GitHub's private vulnerability reporting flow for this repository.

Do not include exploit payloads, crash reproducers with sensitive data, credentials, local paths, or machine-specific information in public issues.

For non-sensitive hardening work, open a regular issue or pull request.

## Expectations

Security fixes should include tests where practical, especially for:

- null/invalid FFI arguments;
- asset path traversal;
- panic-to-status conversion;
- safe handling of unavailable backends.

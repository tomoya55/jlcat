# Security Policy

Thank you for helping to keep this project secure. We take security issues seriously and aim to handle reports responsibly and promptly.

## Supported Versions

Security fixes are provided **only for the latest released version**.

Older versions are not supported. Users are strongly encouraged to upgrade to the most recent release before reporting issues.

## Reporting a Vulnerability

If you discover a security vulnerability, **do not disclose it publicly** (e.g. via GitHub Issues, discussions, or social media).

Please report it privately using one of the following channels:

1. **GitHub Private Vulnerability Reporting (preferred)**  
   Use the “Report a vulnerability” option under the repository’s **Security** tab.
2. **Email**: tomoya@nicecabbage.com  
   Please include `[SECURITY] <project-name>` in the subject line.

### Information to Include

When possible, include the following details to help with triage:

- Affected version or commit hash
- Clear reproduction steps (preferably minimal)
- Expected vs actual behavior
- Impact assessment (e.g. RCE, information disclosure, DoS)
- Proof of concept (only if safe to share)
- Relevant logs or error messages
- Environment details (OS, architecture, Rust version, dependency versions)

Please **do not include real secrets, tokens, or sensitive user data** in your report.

## Response Timeline (Best Effort)

The following timelines are targets, not guarantees:

- **Acknowledgement**: within 3 business days
- **Initial assessment / triage**: within 7 business days
- **Fix plan or mitigation guidance**: within 14 business days

Critical vulnerabilities may be prioritized and handled faster.

## Disclosure Policy

- Vulnerabilities are typically disclosed **after a fix is released**.
- Coordinated disclosure is preferred.
- A standard disclosure window of **up to 90 days** may be used, depending on severity and complexity.
- GitHub Security Advisories may be used for tracking and publication.
- Reporter attribution can be included upon request (anonymous reporting is also acceptable).

## Severity Considerations

Severity is evaluated based on factors such as:

- Remote or local code execution
- Privilege escalation
- Authentication or authorization bypass
- Leakage of sensitive information (keys, tokens, files)
- Supply-chain risks (dependencies, build or release integrity)
- Persistent or hard-to-recover denial of service

## Supply Chain & Build Integrity

This project aims to follow common best practices, including:

- Regular dependency review and vulnerability scanning (e.g. `cargo audit`)
- Reproducible and traceable releases
- CI-based testing and linting

## User Security Notes

- Download binaries only from **official release sources**.
- Avoid running the tool with unnecessary privileges.
- Treat configuration files, environment variables, and logs as potentially sensitive.
- Keep dependencies and the tool itself up to date.

## Contact

- Security contact: tomoya@nicecabbage.com

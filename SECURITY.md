# 🔐 Security Policy

<p>
  <img alt="Security policy" src="https://img.shields.io/badge/security-policy-2563EB?style=for-the-badge&amp;logo=githubsecurity&amp;logoColor=white">
  <img alt="Primary branch: main" src="https://img.shields.io/badge/primary-branch%20main-7C3AED?style=for-the-badge&amp;logo=git&amp;logoColor=white">
</p>

> [!WARNING]
> Do not post exploit details, private recordings, credentials, or personal data in a public issue.

## 🧩 Supported versions

| Version | Security support |
| --- | --- |
| Latest code on `main` | Primary target for security fixes |
| Published releases | May receive fixes when practical |

## 📬 Report a vulnerability

Use [GitHub's private vulnerability reporting page](https://github.com/younsabood/truespeech/security/advisories/new), if it is enabled for this repository. Include the affected version or commit, the impact, and a minimal reproduction when safe to share.

If private reporting is unavailable, contact the repository maintainer through the contact information on the [GitHub profile](https://github.com/younsabood). Please do not disclose an exploitable issue publicly before a fix is available.

## 🔄 Disclosure flow

```mermaid
flowchart LR
    Report["Private report"] --> Triage["Maintainer triage"]
    Triage --> Fix["Prepare and validate fix"]
    Fix --> Coordinate["Coordinate disclosure"]
    Coordinate --> Advisory["Publish advisory when ready"]

    classDef private fill:#ede9fe,stroke:#7c3aed,color:#24134a,stroke-width:2px
    classDef response fill:#dbeafe,stroke:#2563eb,color:#10233f,stroke-width:2px
    classDef resolved fill:#dcfce7,stroke:#16a34a,color:#12351f,stroke-width:2px
    class Report private
    class Triage,Fix,Coordinate response
    class Advisory resolved
```

---

Back to the [project overview](README.md) · Read the [Code of Conduct](CODE_OF_CONDUCT.md).

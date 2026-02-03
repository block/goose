# Security Policy

## Scope

This security policy applies to vulnerabilities discovered in the `main` branch of the following components within the ACI.dev monorepo:

- `/backend`
- `/frontend`

Please report vulnerabilities related to other branches or components if you believe they are critical, but be aware that our primary focus for security patches is the `main` branch of these core components.

## **Reporting a Vulnerability**

We take the security of [ACI.dev](http://ACI.dev) very seriously. If you believe you've found a security vulnerability, please follow these steps:

1. **Do not disclose the vulnerability publicly** or to any third parties.
2. **Minimize Harm:** Make every effort to avoid accessing or downloading data that does not belong to you, disrupting services, or violating user privacy during your testing. If access to user data or confidential information is necessary to demonstrate the vulnerability, please minimize the amount accessed and report this immediately.
3. **Email us directly** at <support@aipolabs.xyz> with
    1. Title format "[Vulnerability] Summary of issue".
    2. Details of the vulnerability in the body.
4. **Include the following information** in your report:
    - Description of the vulnerability
    - Steps to reproduce
    - Potential impact
    - Any suggestions for mitigation
5. We will acknowledge receipt of your vulnerability report within 48 hours and provide an estimated timeline for a fix.
6. Once the vulnerability is fixed, we will notify you and publicly acknowledge your contribution (unless you prefer to remain anonymous).

## **Incident Response Plan (IRP)**

### Purpose

This plan explains how the ACI maintainers handle security incidents. It ensures that issues are confirmed, fixed, and communicated quickly and clearly. It applies to the main branch, covering both the backend and frontend.

### Reporting

All vulnerabilities must be reported by email to <support@aipolabs.xyz>. Public GitHub issues should not be used. Reports should include a description, steps to reproduce, the potential impact, and suggested mitigations. We will acknowledge all reports within 48 hours (or a reasonable timeframe) and provide a timeline for investigation and resolution.

### Process

#### 1. Validation

The first maintainer to review the report becomes the Incident Lead. They confirm if the issue is valid, decide its severity (low, medium, high, or critical), and record which components are affected. If the report is not valid or out of scope, it will be closed.

#### 2. Mitigation

If the issue is valid, we will take steps to reduce immediate risk and then prepare a permanent fix. Once the fix is merged, we release a patched version. The mitigation date, affected components, and link to the fix are recorded.

#### 3. Scoping

We review the project history and logs to check if the issue has been exploited and to understand the scale of impact. If we confirm that user data or systems were affected, we mark the case as a formal incident.

#### 4. Notification

After the issue is resolved, we notify the reporter directly. Reporters may choose to be acknowledged publicly in release notes or remain anonymous. If wider communication is needed, we will publish a GitHub Security Advisory or update the changelog.

### After the Incident

Once resolved, we review what happened, note lessons learned, and improve our processes where possible. We follow a safe harbor policy: researchers who report issues in good faith and follow this process will not face legal action.


## Safe Harbor

We consider security research and vulnerability disclosure activities conducted following this policy to be authorized and beneficial. We will not pursue legal action against individuals who report vulnerabilities in good faith and adhere to this policy, including the restrictions on public disclosure.
This safe harbor does not apply to any actions that intentionally cause harm, disrupt services, violate user privacy, access or modify data beyond what is necessary to demonstrate the vulnerability, or violate any applicable laws.

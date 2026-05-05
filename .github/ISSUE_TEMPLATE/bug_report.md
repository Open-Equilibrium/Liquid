---
name: Bug report
about: Report a defect or unexpected behaviour
title: "[BUG] "
labels: bug
assignees: ''
---

## Summary

<!-- One sentence: what is broken? -->

## Steps to reproduce

<!-- The smallest sequence of commands or actions that reliably triggers the bug. -->

1.
2.
3.

## Expected behaviour

<!-- What did you expect to happen? -->

## Actual behaviour

<!-- What actually happened? Include exact error messages where possible. -->

## Reproducer

<!-- A minimal code snippet, CLI invocation, or test case that demonstrates the
     issue. Smaller is better; the easier it is to reproduce, the more likely
     it gets attention from a spare-time maintainer. -->

```text

```

## Environment

| Field | Value |
|---|---|
| Liquid version / commit SHA | <!-- e.g. 71bd2cb or v0.1.0 --> |
| OS + version | <!-- e.g. Ubuntu 24.04, macOS 15.3, Windows 11 24H2 --> |
| Rust version | <!-- output of `rustc --version` --> |
| Flutter version (if relevant) | <!-- output of `flutter --version` --> |
| Affected layer | Rust core / Flutter shell / SDK / CLI / docs |

## Logs

<!-- Paste relevant log output. If the trace is large, attach it as a file or
     a gist link rather than pasting inline. Sensitive values should be
     redacted. -->

<details><summary>Log excerpt</summary>

```text

```

</details>

## Anything else

<!-- Workarounds you've tried, related issues, anything that helps triage. -->

## Self-checklist

- [ ] I searched existing issues and this is not a duplicate.
- [ ] I am running a recent commit on `main` (or have explained which version).
- [ ] If this is a security issue, I am following [`SECURITY.md`](../../SECURITY.md) instead of filing it here.

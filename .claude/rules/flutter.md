---
paths:
  - "**/*.dart"
  - "app/pubspec.yaml"
  - "app/pubspec.lock"
  - "sdk/liquid_sdk/pubspec.yaml"
  - "sdk/liquid_sdk/pubspec.lock"
  - "apps/**/pubspec.yaml"
  - "app/lib/**"
  - "app/test/**"
  - "app/integration_test/**"
  - "sdk/liquid_sdk/lib/**"
  - "sdk/liquid_sdk/test/**"
  - "sdk/liquid_sdk_lint/**"
  - "apps/**/lib/**"
  - "analysis_options.yaml"
---

# Flutter/Dart Rules

- Prefer idiomatic Dart and the existing project architecture.
- Run `dart format` according to existing conventions
  (project shortcuts: `just fmt-app`, `just fmt-sdk`).
- Run `flutter analyze` for non-trivial changes
  (project shortcuts: `just lint-app`, `just lint-sdk`).
- Add or update widget/unit/integration tests for changed behavior.
- Components render only; **no business logic in Dart** — call into Rust via
  the FFI bridge (per `CLAUDE.md` Step 4).
- Use `AsyncNotifierProvider` (Riverpod) for state backed by an FFI call.
- Avoid broad widget tree rewrites unless requested.
- Avoid unnecessary rebuilds and expensive work in `build()`.
- Preserve existing state-management patterns, localization, theming, routing,
  and accessibility conventions.
- Honor the project's `no_platform_imports` lint: `dart:io`, Flutter plugins,
  and platform channels are banned in `apps/` and `sdk/`
  (per `CLAUDE.md` Absolute Rule 2).
- Honor the project's `no_cross_component_reference` lint: cross-component
  communication goes through `SlotBroker` (per Absolute Rule 3).
- Keep platform-specific changes isolated.
- For UI validation, prefer existing Flutter tests, integration tests, golden
  tests, or screenshots. Use `patrol` for gesture-heavy flows.
- **Do not add Playwright** unless this repo already uses browser e2e tooling.

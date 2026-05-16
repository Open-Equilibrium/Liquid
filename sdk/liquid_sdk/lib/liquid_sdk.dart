/// Public Dart SDK for Liquid components and apps.
///
/// Implements `IMPLEMENTATION_PLAN.md` §6.1 (M8) — the surface
/// app developers extend to build a Liquid app. Re-exports the
/// canonical types from per-feature files so a developer only
/// has to write `import 'package:liquid_sdk/liquid_sdk.dart';`.
///
/// Phase-2 status (M8 — TASK-015):
///
/// - `LiquidComponent`, `InputSlot`, `OutputSlot`, `SlotSchema`
///   — the component-author API, fully typed.
/// - `AppManifest`, `ComponentManifest`, `Permission`,
///   `TenantConfigSchema`, `CliCommandDeclaration` — the
///   declarative manifest types apps emit.
/// - `GridApi`, `VcsApi`, `PermissionApi` — Phase-2 abstract
///   classes; the concrete `flutter_rust_bridge` impls land
///   with TASK-012 (M5 Dart side).
library liquid_sdk;

export 'src/component.dart';
export 'src/manifest.dart';
export 'src/runtime_apis.dart';
export 'src/slot.dart';

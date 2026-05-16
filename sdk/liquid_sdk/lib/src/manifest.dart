/// Declarative manifest types apps emit at install time.
///
/// `AppManifest` is what `liquid app install` reads to discover an
/// app's id, version, components, declared CLI commands, and
/// required permissions (`IMPLEMENTATION_PLAN.md §6.1`). The
/// runtime never instantiates an app without a manifest.
library;

import 'package:flutter/foundation.dart';

import 'slot.dart';

/// Action verb the manifest can request authority for. Mirrors
/// `liquid_core::Action` on the Rust side; values must round-trip
/// through the FFI codegen (TASK-012).
enum ManifestAction { read, write, delete, admin }

/// A permission the app requires at install time. The runtime
/// presents these to the installing user as a consent prompt.
@immutable
class Permission {
  final ManifestAction action;
  final String resourcePattern;
  final String reason;

  const Permission({
    required this.action,
    required this.resourcePattern,
    required this.reason,
  });

  @override
  bool operator ==(Object other) =>
      other is Permission &&
      other.action == action &&
      other.resourcePattern == resourcePattern &&
      other.reason == reason;
  @override
  int get hashCode => Object.hash(action, resourcePattern, reason);
}

/// JSON-Schema-draft-07 fragment describing the tenant config an
/// app instance requires (M10 — `IMPLEMENTATION_PLAN.md §6.3`).
///
/// Phase-2 stores the schema as a `Map<String, dynamic>` — the
/// runtime renders it into a form using a Dart JSON-Schema
/// renderer (the renderer lives in `liquid_sdk_lint` /
/// `app/lib/widgets/`; not the SDK author's concern).
@immutable
class TenantConfigSchema {
  final Map<String, Object?> jsonSchema;

  const TenantConfigSchema({required this.jsonSchema});

  /// Empty schema — the app declares no tenant config (the
  /// runtime skips the install-time form).
  const TenantConfigSchema.empty() : jsonSchema = const {};
}

/// Static declaration of one CLI command the app contributes.
/// The runtime auto-generates `liquid app <instance-name>
/// <verb>` subcommands from these (M7 / TASK-014).
@immutable
class CliCommandDeclaration {
  final String verb;
  final String description;
  final List<String> args;

  const CliCommandDeclaration({
    required this.verb,
    required this.description,
    this.args = const [],
  });
}

/// One component the app ships.
@immutable
class ComponentManifest {
  final String componentId;
  final String displayName;
  final List<InputSlot> inputSlots;
  final List<OutputSlot> outputSlots;
  final int minGridCells;
  final int maxGridCells;
  final List<String> extensionPoints;

  const ComponentManifest({
    required this.componentId,
    required this.displayName,
    this.inputSlots = const [],
    this.outputSlots = const [],
    this.minGridCells = 4,
    this.maxGridCells = 144,
    this.extensionPoints = const [],
  });
}

/// Declarative description of a Liquid app.
///
/// Apps publish one of these (typically alongside a built Flutter
/// package) to the registry. The runtime reads it at install time
/// to verify the app's identity (`id`), version, component set,
/// CLI surface, and required permissions.
@immutable
class AppManifest {
  /// Reverse-DNS id — e.g. `com.example.myapp`.
  final String id;

  /// Semver string.
  final String version;

  /// Schema for the app's per-instance tenant config (M10).
  final TenantConfigSchema tenantConfigSchema;

  /// Every component the app exposes.
  final List<ComponentManifest> components;

  /// Every CLI command the app contributes (M7 / TASK-014).
  final List<CliCommandDeclaration> cliCommands;

  /// Whether other apps may extend this one via declared
  /// extension points.
  final bool supportsExtensions;

  /// Permissions the runtime must grant the app to function.
  final List<Permission> requiredPermissions;

  const AppManifest({
    required this.id,
    required this.version,
    this.tenantConfigSchema = const TenantConfigSchema.empty(),
    this.components = const [],
    this.cliCommands = const [],
    this.supportsExtensions = false,
    this.requiredPermissions = const [],
  });
}

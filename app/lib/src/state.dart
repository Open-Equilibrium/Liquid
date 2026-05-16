/// Phase-2 in-memory state stubs for the M6 shell.
///
/// Real workspace + page state arrives via the FFI bridge
/// (TASK-012). These notifiers match the eventual `AsyncNotifier`
/// shape so swapping in the FFI impl later is a typedef change,
/// not a widget rewrite.

library;

import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// Mirrors `liquid_sdk_bridge::WorkspaceSummary`. Phase-2 stub
/// uses Dart primitives so the shell compiles without the bridge.
@immutable
class WorkspaceSummary {
  final String id;
  final String name;
  const WorkspaceSummary({required this.id, required this.name});

  @override
  bool operator ==(Object other) =>
      other is WorkspaceSummary && other.id == id && other.name == name;
  @override
  int get hashCode => Object.hash(id, name);
}

/// Demo workspaces the M6 shell shows before the FFI bridge wires
/// real state in.
final workspacesProvider = StateProvider<List<WorkspaceSummary>>((_) => const [
      WorkspaceSummary(id: 'demo-1', name: 'Personal'),
      WorkspaceSummary(id: 'demo-2', name: 'Team'),
    ]);

/// Index of the currently-selected workspace into
/// `workspacesProvider`. `null` if no workspaces exist.
final currentWorkspaceProvider = StateProvider<int?>((ref) {
  final ws = ref.watch(workspacesProvider);
  return ws.isEmpty ? null : 0;
});

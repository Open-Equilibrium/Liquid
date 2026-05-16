/// Runtime APIs the host injects into every component at mount
/// time. Phase-2 ships the abstract classes; the concrete
/// `flutter_rust_bridge`-backed implementations land with
/// TASK-012 (M5 Dart side — FFI codegen + concrete `GridApi`
/// / `VcsApi` / `PermissionApi`) and TASK-016b (M9 Dart side —
/// slot emitter / consumer wired through the broker).
library;

import 'dart:async';

import 'manifest.dart';
import 'slot.dart';

/// Page-grid resize / maximise hooks.
abstract class GridApi {
  /// Ask the host to resize the component's grid cell to
  /// `columns × rows`. The host may reject if it would violate
  /// the component's declared [GridConstraints].
  Future<void> requestResize({required int columns, required int rows});

  /// Ask the host to expand the component to fill the entire
  /// `PageArea`. A second call toggles back to the previous
  /// size.
  Future<void> requestMaximise();
}

/// VCS-backed read / write / history scoped to the component's
/// current app instance. Phase-2 contract; the concrete impl
/// goes through the bridge's `read_page` / `write_page` FFI
/// (TASK-012).
abstract class VcsApi {
  /// Read the current bytes at `path` in this component's
  /// workspace + app-instance scope.
  Future<List<int>> read(String path);

  /// Atomically write `bytes` to `path` with a commit `message`.
  Future<String> write(String path, List<int> bytes, {required String message});

  /// Paginated operation-log entries that touched `path`,
  /// newest-first.
  Future<List<HistoryEntry>> history(String path, {int limit = 50});

  /// Reverse a prior operation (by id).
  Future<String> undo(String operationId);

  /// The tenant config the host loaded for this instance (M10).
  /// `null` if the app declared no tenant schema.
  Map<String, Object?>? get tenantConfig;
}

/// One row in the operation-log history view.
class HistoryEntry {
  final String operationId;
  final String commitId;
  final int timestampUnixMillis;
  final String principal;
  final String action;
  final String path;
  final String message;
  const HistoryEntry({
    required this.operationId,
    required this.commitId,
    required this.timestampUnixMillis,
    required this.principal,
    required this.action,
    required this.path,
    required this.message,
  });
}

/// Permission-query API. Components ask whether the active
/// principal may perform `action` on `resource` BEFORE rendering
/// destructive UI affordances (e.g. show the trash icon only
/// when delete is permitted).
abstract class PermissionApi {
  Future<bool> check(
      {required ManifestAction action, required String resource});
}

/// Live publish/subscribe handle for an [OutputSlot]. Phase-2
/// abstract; the concrete impl wraps the bridge's
/// `publish_slot` / `subscribe_slot` FFI (TASK-016b — M9 Dart
/// side, blocked on TASK-012 codegen).
abstract class SlotEmitter {
  Future<int> emit(SlotValue value);
}

/// Live subscription handle for an [InputSlot]. Phase-2
/// abstract; concrete impl returns a `Stream<SlotValue>` backed
/// by the bridge's `subscribe_slot` FFI.
abstract class SlotConsumer {
  Stream<SlotValue> get stream;
}

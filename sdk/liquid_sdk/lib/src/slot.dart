/// Typed slot value + schema. Mirrors `liquid_core::SlotValue` on
/// the Rust side (`IMPLEMENTATION_PLAN.md` §6.1 / §13).
library;

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';

const _jsonEq = DeepCollectionEquality();

/// Discriminated union over the value a slot can carry.
///
/// Round-trips with `liquid_core::SlotValue` via the
/// `flutter_rust_bridge` codegen (TASK-012). Use the typed
/// constructors and the `when` matcher for exhaustive consumers.
@immutable
sealed class SlotValue {
  const SlotValue();

  /// String value.
  const factory SlotValue.str(String value) = _Str;

  /// Numeric value (double — matches Dart's default Number type
  /// and the Rust-side `SlotValue::Num(f64)`).
  const factory SlotValue.num(double value) = _Num;

  /// Boolean value.
  const factory SlotValue.bool(bool value) = _Bool;

  /// Arbitrary JSON value (`Map<String, dynamic>` / `List<dynamic>`
  /// / primitive).
  const factory SlotValue.json(Object value) = _Json;

  /// Raw byte buffer (e.g. binary blob).
  const factory SlotValue.bytes(List<int> value) = _Bytes;

  /// Exhaustive consumer — pattern-match on the variant.
  T when<T>({
    required T Function(String value) str,
    required T Function(double value) num,
    required T Function(bool value) boolean,
    required T Function(Object value) json,
    required T Function(List<int> value) bytes,
  });
}

class _Str extends SlotValue {
  final String value;
  const _Str(this.value);
  @override
  T when<T>({
    required T Function(String value) str,
    required T Function(double value) num,
    required T Function(bool value) boolean,
    required T Function(Object value) json,
    required T Function(List<int> value) bytes,
  }) =>
      str(value);
  @override
  bool operator ==(Object other) => other is _Str && other.value == value;
  @override
  int get hashCode => value.hashCode;
  @override
  String toString() => 'SlotValue.str($value)';
}

class _Num extends SlotValue {
  final double value;
  const _Num(this.value);
  @override
  T when<T>({
    required T Function(String value) str,
    required T Function(double value) num,
    required T Function(bool value) boolean,
    required T Function(Object value) json,
    required T Function(List<int> value) bytes,
  }) =>
      num(value);
  @override
  bool operator ==(Object other) => other is _Num && other.value == value;
  @override
  int get hashCode => value.hashCode;
  @override
  String toString() => 'SlotValue.num($value)';
}

class _Bool extends SlotValue {
  final bool value;
  const _Bool(this.value);
  @override
  T when<T>({
    required T Function(String value) str,
    required T Function(double value) num,
    required T Function(bool value) boolean,
    required T Function(Object value) json,
    required T Function(List<int> value) bytes,
  }) =>
      boolean(value);
  @override
  bool operator ==(Object other) => other is _Bool && other.value == value;
  @override
  int get hashCode => value.hashCode;
  @override
  String toString() => 'SlotValue.bool($value)';
}

class _Json extends SlotValue {
  final Object value;
  const _Json(this.value);
  @override
  T when<T>({
    required T Function(String value) str,
    required T Function(double value) num,
    required T Function(bool value) boolean,
    required T Function(Object value) json,
    required T Function(List<int> value) bytes,
  }) =>
      json(value);
  @override
  bool operator ==(Object other) =>
      other is _Json && _jsonEq.equals(other.value, value);
  @override
  int get hashCode => _jsonEq.hash(value);
  @override
  String toString() => 'SlotValue.json($value)';
}

class _Bytes extends SlotValue {
  final List<int> value;
  const _Bytes(this.value);
  @override
  T when<T>({
    required T Function(String value) str,
    required T Function(double value) num,
    required T Function(bool value) boolean,
    required T Function(Object value) json,
    required T Function(List<int> value) bytes,
  }) =>
      bytes(value);
  @override
  bool operator ==(Object other) =>
      other is _Bytes && listEquals(other.value, value);
  @override
  int get hashCode => Object.hashAll(value);
  @override
  String toString() => 'SlotValue.bytes(${value.length} bytes)';
}

/// Type tag for a slot — emitted in [`ComponentManifest`]'s
/// `inputSlots` / `outputSlots` so the runtime can validate
/// wiring (an `OutputSlot<num>` can wire to an `InputSlot<num>`
/// but not to an `InputSlot<bool>`).
enum SlotKind { str, num, bool_, json, bytes }

/// JSON-schema-style descriptor for a slot value. Phase-2 only
/// stores the [SlotKind] tag; future revisions add JSON-Schema-
/// fragment validation for the `json` arm.
@immutable
class SlotSchema {
  final SlotKind kind;
  final String description;

  const SlotSchema({required this.kind, this.description = ''});

  @override
  bool operator ==(Object other) =>
      other is SlotSchema &&
      other.kind == kind &&
      other.description == description;
  @override
  int get hashCode => Object.hash(kind, description);
}

/// One end of a wireable slot. Components hold these in their
/// `inputs` / `outputs` maps. Phase-2 ships the typed handles
/// only; the live `Stream` / `emit` are wired into the
/// `flutter_rust_bridge` SlotBroker FFI by TASK-012.
@immutable
class InputSlot {
  final String name;
  final SlotSchema schema;
  const InputSlot({required this.name, required this.schema});
}

@immutable
class OutputSlot {
  final String name;
  final SlotSchema schema;
  const OutputSlot({required this.name, required this.schema});
}

/// Convenience aliases — components return these from their
/// `inputs` / `outputs` getters.
typedef InputSlotMap = Map<String, InputSlot>;
typedef OutputSlotMap = Map<String, OutputSlot>;

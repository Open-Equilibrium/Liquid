/// `LiquidComponent` — the abstract base every component author
/// extends.
library;

import 'package:flutter/widgets.dart';

import 'slot.dart';

/// Constraints a component declares for the page-grid host —
/// minimum and maximum grid-cell footprint (12-column default
/// per `IMPLEMENTATION_PLAN.md §10.3`).
@immutable
class GridConstraints {
  final int minColumns;
  final int minRows;
  final int maxColumns;
  final int maxRows;

  const GridConstraints({
    required this.minColumns,
    required this.minRows,
    required this.maxColumns,
    required this.maxRows,
  })  : assert(minColumns >= 1),
        assert(minRows >= 1),
        assert(maxColumns >= minColumns),
        assert(maxRows >= minRows);

  /// Reasonable default for components that don't have strict
  /// size requirements.
  static const GridConstraints flexible =
      GridConstraints(minColumns: 2, minRows: 2, maxColumns: 12, maxRows: 12);
}

/// Abstract base for every Liquid component. Concrete
/// implementations are normal Flutter `StatefulWidget`s that
/// additionally:
///
/// - Declare their `inputs` and `outputs` ([SlotName] → typed
///   [InputSlot] / [OutputSlot]).
/// - Declare their `gridConstraints` for layout.
///
/// The runtime hosts the component inside the `PageGrid` and
/// wires its declared slots to the active page's
/// [`BindingsDocument`] (see `liquid_bindings::BindingsDocument`).
///
/// Example:
///
/// ```dart
/// class CounterComponent extends LiquidComponent {
///   const CounterComponent({super.key});
///
///   @override
///   InputSlotMap get inputs => {
///     'reset': InputSlot(
///       name: 'reset',
///       schema: SlotSchema(kind: SlotKind.bool_),
///     ),
///   };
///
///   @override
///   OutputSlotMap get outputs => {
///     'count': OutputSlot(
///       name: 'count',
///       schema: SlotSchema(kind: SlotKind.num),
///     ),
///   };
///
///   @override
///   GridConstraints get gridConstraints => GridConstraints.flexible;
///
///   @override
///   State<CounterComponent> createState() => _CounterState();
/// }
/// ```
abstract class LiquidComponent extends StatefulWidget {
  const LiquidComponent({super.key});

  /// Slots this component CONSUMES. Wiring an `OutputSlot` to
  /// one of these connects the component to upstream data.
  InputSlotMap get inputs;

  /// Slots this component PRODUCES. Wiring one of these to an
  /// `InputSlot` lets downstream components react.
  OutputSlotMap get outputs;

  /// Grid-cell footprint the component prefers + tolerates.
  GridConstraints get gridConstraints;
}

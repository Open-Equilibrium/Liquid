// ignore_for_file: dangling_library_doc_comments
// ignore_for_file: prefer_const_constructors
// ignore_for_file: prefer_const_literals_to_create_immutables
/// M8 plan-level success criterion
/// (`IMPLEMENTATION_PLAN.md §6.1`):
///
/// > A developer can create a new Flutter package, depend on
/// > `liquid_sdk`, extend `LiquidComponent`, declare two slots,
/// > and the SDK compiles with no errors.
///
/// We exercise that criterion in-package: define a stub
/// component with one input + one output, confirm it compiles
/// and its declared surface round-trips through the API.

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:liquid_sdk/liquid_sdk.dart';

class _ResetCounter extends LiquidComponent {
  const _ResetCounter();

  @override
  InputSlotMap get inputs => const {
        'reset': InputSlot(
          name: 'counter:reset',
          schema: SlotSchema(kind: SlotKind.bool_),
        ),
      };

  @override
  OutputSlotMap get outputs => const {
        'count': OutputSlot(
          name: 'counter:count',
          schema: SlotSchema(kind: SlotKind.num),
        ),
      };

  @override
  GridConstraints get gridConstraints => GridConstraints.flexible;

  @override
  State<_ResetCounter> createState() => _ResetCounterState();
}

class _ResetCounterState extends State<_ResetCounter> {
  @override
  Widget build(BuildContext context) => const Text('counter');
}

void main() {
  group('LiquidComponent (M8 success criterion)', () {
    test('declared inputs + outputs round-trip', () {
      const component = _ResetCounter();
      expect(component.inputs.keys, contains('reset'));
      expect(component.outputs.keys, contains('count'));
      expect(component.inputs['reset']!.schema.kind, SlotKind.bool_);
      expect(component.outputs['count']!.schema.kind, SlotKind.num);
    });

    test('grid constraints honour declared minimums', () {
      const c = _ResetCounter();
      expect(c.gridConstraints.minColumns, greaterThanOrEqualTo(1));
      expect(c.gridConstraints.maxColumns,
          greaterThanOrEqualTo(c.gridConstraints.minColumns));
    });
  });

  group('SlotValue typed variants', () {
    test('str matcher routes to the str arm', () {
      const v = SlotValue.str('hello');
      final routed = v.when(
        str: (s) => 'str:$s',
        num: (_) => 'num',
        boolean: (_) => 'bool',
        json: (_) => 'json',
        bytes: (_) => 'bytes',
      );
      expect(routed, 'str:hello');
    });

    test('num matcher routes to the num arm', () {
      const v = SlotValue.num(3.14);
      final routed = v.when(
        str: (_) => 'str',
        num: (n) => 'num:$n',
        boolean: (_) => 'bool',
        json: (_) => 'json',
        bytes: (_) => 'bytes',
      );
      expect(routed, 'num:3.14');
    });

    test('equality holds for identical str values', () {
      expect(const SlotValue.str('a'), const SlotValue.str('a'));
      expect(const SlotValue.str('a') == const SlotValue.str('b'), isFalse);
    });

    test('json equality is structural, not identity', () {
      // Use runtime (`final`) maps + lists — `const` literals would
      // be canonicalised by Dart to the same instance, so the
      // pre-fix identity-based operator== would have wrongly passed.
      // Building fresh `Map` / `List` objects forces the deep-equality
      // path that DeepCollectionEquality is meant to handle. The
      // `identical(a, b)` assertion below is the explicit guard
      // against the test regressing back into the canonicalised form.
      final a = SlotValue.json(<String, Object>{
        'k': 1,
        'nested': <int>[1, 2, 3],
      });
      final b = SlotValue.json(<String, Object>{
        'k': 1,
        'nested': <int>[1, 2, 3],
      });
      final c = SlotValue.json(<String, Object>{
        'k': 1,
        'nested': <int>[1, 2, 4],
      });
      expect(identical(a, b), isFalse,
          reason: 'sanity: a and b must be distinct objects so the '
              'test exercises structural equality, not identity');
      expect(a, b,
          reason: 'two json values with deep-equal content must be ==');
      expect(a.hashCode, b.hashCode, reason: 'hashCode must agree with ==');
      expect(a == c, isFalse, reason: 'differing leaf must break equality');
    });

    test('bytes equality is structural', () {
      // Same canonicalisation hazard as above — use `final` so each
      // call site produces a fresh `_Bytes` instance.
      final a = SlotValue.bytes(<int>[1, 2, 3]);
      final b = SlotValue.bytes(<int>[1, 2, 3]);
      final c = SlotValue.bytes(<int>[1, 2, 4]);
      expect(identical(a, b), isFalse);
      expect(a, b);
      expect(a == c, isFalse);
    });
  });

  group('AppManifest', () {
    test('round-trips an empty schema and a single permission', () {
      const manifest = AppManifest(
        id: 'com.example.test',
        version: '0.1.0',
        requiredPermissions: [
          Permission(
            action: ManifestAction.read,
            resourcePattern: 'workspace/*',
            reason: 'list workspaces',
          ),
        ],
      );
      expect(manifest.id, 'com.example.test');
      expect(manifest.tenantConfigSchema.jsonSchema, isEmpty);
      expect(manifest.requiredPermissions.single.action, ManifestAction.read);
    });
  });
}

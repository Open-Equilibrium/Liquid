/// Liquid desktop shell — M6 (`IMPLEMENTATION_PLAN.md §5.7`).
///
/// Implements the four canonical widgets — `RootShell`,
/// `ExplorerPanel`, `PageArea`, `PageGrid` — plus a placeholder
/// `GridItem` so the grid is exercisable before real app
/// instances exist (the success criterion for M6).
///
/// Riverpod hosts every state container. Workspace + page state
/// arrives via the FFI bridge (TASK-012 / M5 Dart side), so
/// Phase-2 ships in-memory stubs that match the eventual FFI
/// shape — the swap is a notifier-impl swap, not a widget
/// rewrite.

library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'src/root_shell.dart';

// Re-export shell-internal widgets so widget tests can import
// them via `package:liquid_app/main.dart`.
export 'src/explorer_panel.dart' show ExplorerPanel;
export 'src/page_area.dart' show PageArea;
export 'src/page_grid.dart' show GridItem, PageGrid;
export 'src/root_shell.dart' show RootShell;
export 'src/state.dart'
    show WorkspaceSummary, currentWorkspaceProvider, workspacesProvider;

void main() {
  runApp(const ProviderScope(child: LiquidApp()));
}

class LiquidApp extends StatelessWidget {
  const LiquidApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
        title: 'Liquid',
        theme: ThemeData(
          colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
          useMaterial3: true,
        ),
        home: const RootShell(),
      );
}

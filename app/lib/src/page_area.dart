/// `PageArea` — the right-hand region. Hosts the page toolbar
/// and the `PageGrid` for the currently-selected workspace.
///
/// Phase-2 stub: empty toolbar verbs (`save`, `history`, `share`
/// land with M7 / TASK-009 + M8 follow-ups); the grid below is
/// a real M6 widget with a placeholder grid item.

library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'page_grid.dart';
import 'state.dart';

class PageArea extends ConsumerWidget {
  const PageArea({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final workspaces = ref.watch(workspacesProvider);
    final selected = ref.watch(currentWorkspaceProvider);
    final title = (selected != null && selected < workspaces.length)
        ? workspaces[selected].name
        : 'No workspace selected';

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _Toolbar(title: title),
        const Divider(height: 1),
        const Expanded(child: PageGrid()),
      ],
    );
  }
}

class _Toolbar extends StatelessWidget {
  final String title;
  const _Toolbar({required this.title});

  @override
  Widget build(BuildContext context) => Container(
        color: Theme.of(context).colorScheme.surfaceContainerLowest,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        child: Row(
          children: [
            Expanded(
              child: Text(title,
                  style: Theme.of(context).textTheme.titleMedium),
            ),
            IconButton(
              key: const Key('toolbar-add-item'),
              icon: const Icon(Icons.add),
              tooltip: 'Add item',
              onPressed: () {},
            ),
            const IconButton(
              key: Key('toolbar-save'),
              icon: Icon(Icons.save_outlined),
              tooltip: 'Save (pending M8)',
              onPressed: null,
            ),
            const IconButton(
              key: Key('toolbar-history'),
              icon: Icon(Icons.history),
              tooltip: 'History (pending M8)',
              onPressed: null,
            ),
          ],
        ),
      );
}

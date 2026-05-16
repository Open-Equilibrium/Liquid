/// `ExplorerPanel` — workspace switcher (compact dropdown at the
/// top) + a placeholder Pages / Apps / Tags section list.
///
/// Phase-2 stub: the dropdown reads `workspacesProvider` from
/// `state.dart`; the section content is placeholders. M8+ wires
/// real `PageTreeView` / `AppInstanceListView` / `TagSectionView`
/// children once those data sources land.

library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'state.dart';

class ExplorerPanel extends ConsumerWidget {
  const ExplorerPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final workspaces = ref.watch(workspacesProvider);
    final selected = ref.watch(currentWorkspaceProvider);

    return Container(
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(12, 12, 12, 8),
            child: _WorkspaceSwitcher(
              workspaces: workspaces,
              selectedIndex: selected,
              onChanged: (i) =>
                  ref.read(currentWorkspaceProvider.notifier).state = i,
            ),
          ),
          const Divider(height: 1),
          Expanded(
            child: ListView(
              padding: const EdgeInsets.symmetric(vertical: 8),
              children: const [
                _SectionHeader('Pages'),
                _Placeholder('Page tree (M6.5+ — TASK-013)'),
                _SectionHeader('Apps'),
                _Placeholder('App instances (M8+ — TASK-015)'),
                _SectionHeader('Tags'),
                _Placeholder('Tag filter rules (M6+ follow-up)'),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _WorkspaceSwitcher extends StatelessWidget {
  final List<WorkspaceSummary> workspaces;
  final int? selectedIndex;
  final ValueChanged<int?> onChanged;
  const _WorkspaceSwitcher({
    required this.workspaces,
    required this.selectedIndex,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    if (workspaces.isEmpty) {
      return const Text('No workspaces',
          style: TextStyle(fontStyle: FontStyle.italic));
    }
    return DropdownButton<int>(
      key: const Key('workspace-switcher'),
      isExpanded: true,
      value: selectedIndex,
      items: [
        for (var i = 0; i < workspaces.length; i++)
          DropdownMenuItem(value: i, child: Text(workspaces[i].name)),
      ],
      onChanged: onChanged,
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String text;
  const _SectionHeader(this.text);

  @override
  Widget build(BuildContext context) => Padding(
        padding: const EdgeInsets.fromLTRB(16, 12, 16, 4),
        child: Text(
          text.toUpperCase(),
          style: Theme.of(context)
              .textTheme
              .labelSmall
              ?.copyWith(letterSpacing: 1.2),
        ),
      );
}

class _Placeholder extends StatelessWidget {
  final String text;
  const _Placeholder(this.text);

  @override
  Widget build(BuildContext context) => ListTile(
        dense: true,
        title: Text(text,
            style: const TextStyle(fontStyle: FontStyle.italic, fontSize: 13)),
      );
}

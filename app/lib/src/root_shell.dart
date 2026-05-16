/// `RootShell` — top-level layout: a resizable `ExplorerPanel`
/// on the left and a `PageArea` filling the remaining space.

library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'explorer_panel.dart';
import 'page_area.dart';

class RootShell extends ConsumerStatefulWidget {
  const RootShell({super.key});

  @override
  ConsumerState<RootShell> createState() => _RootShellState();
}

class _RootShellState extends ConsumerState<RootShell> {
  static const double _minExplorerWidth = 200;
  static const double _maxExplorerWidth = 480;
  double _explorerWidth = 260;

  @override
  Widget build(BuildContext context) => Scaffold(
        body: Row(
          children: [
            SizedBox(
              width: _explorerWidth,
              child: const ExplorerPanel(),
            ),
            MouseRegion(
              cursor: SystemMouseCursors.resizeLeftRight,
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onHorizontalDragUpdate: (d) {
                  setState(() {
                    _explorerWidth = (_explorerWidth + d.delta.dx)
                        .clamp(_minExplorerWidth, _maxExplorerWidth);
                  });
                },
                child: Container(
                  width: 6,
                  color: Theme.of(context).dividerColor,
                ),
              ),
            ),
            const Expanded(child: PageArea()),
          ],
        ),
      );
}

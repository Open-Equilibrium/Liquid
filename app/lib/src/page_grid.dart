/// `PageGrid` — a 12-column / variable-row layout that hosts
/// `GridItem` children (each wraps an app instance or component).
///
/// Phase-2 implementation:
///
/// - Fixed 12-column / 12-row coordinate system; cell size
///   recomputes from the available canvas (no scrolling — pages
///   wider/taller than the viewport are an M6+ follow-up).
/// - One placeholder `GridItem` (coloured box) renders by default
///   so the grid is interactive before real app instances exist
///   — the M6 success criterion.
/// - Drag-to-reposition via long-press + drag; snap-to-grid on
///   release.
/// - Bottom-right resize handle drags height + width by one cell
///   per pixel-threshold; constrained to within the 12×12 grid.

library;

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

/// One occupant of the page grid. Phase-2 stub: just a coloured
/// box with a name. Real component hosting (`LiquidComponent`)
/// lands with M8 + the bridge's `load_page` FFI.
@immutable
class GridItem {
  final String id;
  final String label;
  final Color color;
  final int column;
  final int row;
  final int columnSpan;
  final int rowSpan;

  const GridItem({
    required this.id,
    required this.label,
    required this.color,
    required this.column,
    required this.row,
    this.columnSpan = 3,
    this.rowSpan = 3,
  });

  GridItem copyWith({
    int? column,
    int? row,
    int? columnSpan,
    int? rowSpan,
  }) =>
      GridItem(
        id: id,
        label: label,
        color: color,
        column: column ?? this.column,
        row: row ?? this.row,
        columnSpan: columnSpan ?? this.columnSpan,
        rowSpan: rowSpan ?? this.rowSpan,
      );

  @override
  bool operator ==(Object other) =>
      other is GridItem &&
      other.id == id &&
      other.column == column &&
      other.row == row &&
      other.columnSpan == columnSpan &&
      other.rowSpan == rowSpan;

  @override
  int get hashCode => Object.hash(id, column, row, columnSpan, rowSpan);
}

/// Grid items currently on the active page. Phase-2 seeds one
/// placeholder so a fresh launch exercises the grid widget.
final gridItemsProvider = StateProvider<List<GridItem>>((_) => const [
      GridItem(
        id: 'placeholder',
        label: 'Placeholder',
        color: Color(0xFF6750A4),
        column: 1,
        row: 1,
      ),
    ]);

const int kPageGridColumns = 12;
const int kPageGridRows = 12;

class PageGrid extends ConsumerWidget {
  const PageGrid({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final items = ref.watch(gridItemsProvider);
    return LayoutBuilder(
      key: const Key('page-grid'),
      builder: (context, constraints) {
        final cellWidth = constraints.maxWidth / kPageGridColumns;
        final cellHeight = constraints.maxHeight / kPageGridRows;
        return Container(
          color: Theme.of(context).colorScheme.surface,
          child: Stack(
            children: [
              for (final item in items)
                _PositionedGridItem(
                  key: Key('grid-item-${item.id}'),
                  item: item,
                  cellWidth: cellWidth,
                  cellHeight: cellHeight,
                  onMove: (newCol, newRow) {
                    ref.read(gridItemsProvider.notifier).state = items
                        .map((it) => it.id == item.id
                            ? it.copyWith(column: newCol, row: newRow)
                            : it)
                        .toList();
                  },
                  onResize: (newCols, newRows) {
                    ref.read(gridItemsProvider.notifier).state = items
                        .map((it) => it.id == item.id
                            ? it.copyWith(columnSpan: newCols, rowSpan: newRows)
                            : it)
                        .toList();
                  },
                ),
            ],
          ),
        );
      },
    );
  }
}

class _PositionedGridItem extends StatelessWidget {
  final GridItem item;
  final double cellWidth;
  final double cellHeight;
  final void Function(int column, int row) onMove;
  final void Function(int columnSpan, int rowSpan) onResize;

  const _PositionedGridItem({
    super.key,
    required this.item,
    required this.cellWidth,
    required this.cellHeight,
    required this.onMove,
    required this.onResize,
  });

  @override
  Widget build(BuildContext context) {
    final left = (item.column - 1) * cellWidth;
    final top = (item.row - 1) * cellHeight;
    final width = item.columnSpan * cellWidth;
    final height = item.rowSpan * cellHeight;

    return Positioned(
      left: left,
      top: top,
      width: width,
      height: height,
      child: Stack(
        children: [
          GestureDetector(
            behavior: HitTestBehavior.translucent,
            onPanUpdate: (d) {
              final newCol = ((left + d.delta.dx) / cellWidth).round() + 1;
              final newRow = ((top + d.delta.dy) / cellHeight).round() + 1;
              final clampedCol =
                  newCol.clamp(1, kPageGridColumns - item.columnSpan + 1);
              final clampedRow =
                  newRow.clamp(1, kPageGridRows - item.rowSpan + 1);
              if (clampedCol != item.column || clampedRow != item.row) {
                onMove(clampedCol, clampedRow);
              }
            },
            child: Container(
              margin: const EdgeInsets.all(4),
              decoration: BoxDecoration(
                // ~0.18 alpha — Color.withOpacity is deprecated in
                // Flutter 3.27+; withAlpha is the stable cross-version
                // replacement that keeps integer alpha precision.
                color: item.color.withAlpha(46),
                border: Border.all(color: item.color, width: 1),
                borderRadius: BorderRadius.circular(8),
              ),
              alignment: Alignment.center,
              child: Text(item.label,
                  style: TextStyle(
                      color: item.color, fontWeight: FontWeight.w500)),
            ),
          ),
          Positioned(
            right: 0,
            bottom: 0,
            child: GestureDetector(
              behavior: HitTestBehavior.opaque,
              onPanUpdate: (d) {
                final newCols = ((width + d.delta.dx) / cellWidth).round();
                final newRows = ((height + d.delta.dy) / cellHeight).round();
                final clampedCols =
                    newCols.clamp(1, kPageGridColumns - item.column + 1);
                final clampedRows =
                    newRows.clamp(1, kPageGridRows - item.row + 1);
                if (clampedCols != item.columnSpan ||
                    clampedRows != item.rowSpan) {
                  onResize(clampedCols, clampedRows);
                }
              },
              child: MouseRegion(
                cursor: SystemMouseCursors.resizeDownRight,
                child: Container(
                  width: 16,
                  height: 16,
                  color: item.color,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

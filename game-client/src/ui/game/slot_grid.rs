//! 统一间距网格布局：计算 cols×rows 使水平/垂直间距近似相等

use bevy_egui::egui;

/// 槽位统一尺寸（像素）
pub const SLOT_SIZE: f32 = 72.0;
/// 槽位最小间距（像素）
pub const SLOT_GAP: f32 = 12.0;

/// 网格配置（简化版，仅标识和拖拽开关）
pub struct SlotGridConfig {
    pub id: String,
    pub dnd_enabled: bool,
}

/// 统一间距网格布局：在 w×h 区域中放置 ss×ss 的单元格，
/// 寻找 (cols, rows) 使水平间距与垂直间距尽量相等。
/// 返回 (cols, rows, unified_gap)。
pub fn compute_unified_layout(w: f32, h: f32, ss: f32, min_gap: f32) -> (usize, usize, f32) {
    let max_cols = ((w + min_gap) / (ss + min_gap)).floor().max(1.0) as usize;
    let mut best = (1_usize, 1_usize, min_gap);
    let mut best_diff = f32::MAX;

    for cols in (1..=max_cols).rev() {
        let h_gap = if cols > 1 {
            (w - cols as f32 * ss) / (cols as f32 - 1.0)
        } else {
            min_gap
        };
        if h_gap < min_gap {
            break;
        }

        // 用 min_gap 算最大行数（保守），然后验证
        let max_rows = ((h + min_gap) / (ss + min_gap)).floor().max(1.0) as usize;
        // 尝试 max_rows 和 max_rows-1，取 diff 最小的
        // 找最多行数，v_gap >= min_gap 且与 h_gap 差最小
        // 优先多行（r=1 只在无其他选择时使用）
        let mut best_rows = max_rows;
        let mut best_v = if max_rows > 1 {
            (h - max_rows as f32 * ss) / (max_rows as f32 - 1.0)
        } else {
            h_gap
        };
        let mut best_row_diff = f32::MAX;
        for r in (2..=max_rows).rev() {
            let vg = (h - r as f32 * ss) / (r as f32 - 1.0);
            if vg < min_gap {
                continue;
            }
            let d = (h_gap - vg).abs();
            if d < best_row_diff {
                best_row_diff = d;
                best_rows = r;
                best_v = vg;
            }
        }
        let rows = best_rows;
        let v_gap = best_v;
        if v_gap < min_gap {
            continue;
        }

        let diff = (h_gap - v_gap).abs();
        if diff < best_diff {
            best_diff = diff;
            best = (cols, rows, (h_gap + v_gap) / 2.0);
        }
    }
    best
}

/// 根据网格坐标和 span 计算单元格矩形。
/// origin 为网格左上角绝对坐标，col/row 为起始列/行索引。
pub fn cell_rect(
    origin: egui::Pos2,
    col: usize,
    row: usize,
    col_span: usize,
    row_span: usize,
    ss: f32,
    gap: f32,
) -> egui::Rect {
    let x = col as f32 * (ss + gap);
    let y = row as f32 * (ss + gap);
    let w = col_span as f32 * ss + (col_span.saturating_sub(1)) as f32 * gap;
    let h = row_span as f32 * ss + (row_span.saturating_sub(1)) as f32 * gap;
    egui::Rect::from_min_size(origin + egui::vec2(x, y), egui::vec2(w, h))
}

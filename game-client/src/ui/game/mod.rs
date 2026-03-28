//! 游戏 UI 业务组件：槽位、网格、武器信息、统计条等

pub mod disc_info;
pub mod slot;
pub mod slot_grid;
pub mod stat_bar;
pub mod stat_row;
pub mod trash_slot;

pub use slot::SlotLoc;

// 以下 re-export 在 Phase 4 统一替换后会被外部模块使用
#[allow(unused_imports)]
pub use disc_info::disc_info;
#[allow(unused_imports)]
pub use slot::{SlotContent, SlotResponse};
#[allow(unused_imports)]
pub use slot_grid::{cell_rect, compute_unified_layout, SlotGridConfig, SLOT_GAP, SLOT_SIZE};
#[allow(unused_imports)]
pub use stat_bar::stat_bar;
#[allow(unused_imports)]
pub use stat_row::{stat_label, stat_row};
#[allow(unused_imports)]
pub use trash_slot::trash_slot;

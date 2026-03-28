//! 拖拽系统：数据定义、Disc 替换逻辑

use bevy::prelude::*;

use super::{ClientInventory, ClientItem, ClientProgram};
use crate::GameState;

// ─── 拖拽来源 ───

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DragOrigin {
    /// 从背包格子拖出
    Bag { index: usize },
    /// 从 Disc 槽位拖出
    DiscSlot { index: usize },
}

// ─── 拖拽中携带的数据 ───

#[derive(Debug, Clone)]
pub struct DragItem {
    pub origin: DragOrigin,
    pub item: DragPayload,
}

#[derive(Debug, Clone)]
pub enum DragPayload {
    /// 背包物品（Disc 或 Program）
    BagItem(ClientItem),
    /// Disc 槽位中的 Program
    DiscProgram(ClientProgram),
}

// ─── DragState 资源 ───

#[derive(Resource, Default, Debug)]
pub struct DragState {
    pub dragging: Option<DragItem>,
}

// ─── 插件 ───

pub struct DragPlugin;

impl Plugin for DragPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DragState>()
            .add_systems(Update, drag_system.run_if(in_state(GameState::Playing)));
    }
}

/// 拖拽逻辑（预留，具体交互在 egui 面板系统中处理）
fn drag_system(_drag_state: Res<DragState>) {}

/// 处理拖拽交换，返回无法放入背包的被丢弃 Programs
pub fn handle_drop(
    inv: &mut ClientInventory,
    from: crate::ui::game::SlotLoc,
    to: crate::ui::game::SlotLoc,
) -> Vec<ClientProgram> {
    use crate::ui::game::SlotLoc;
    if from == to {
        return Vec::new();
    }
    match (from, to) {
        // Program: 背包 -> Disc 槽位
        (SlotLoc::Bag(bi), SlotLoc::DiscSlot(di)) => {
            let Some(disc) = &mut inv.equipped_disc else {
                return Vec::new();
            };
            if di >= disc.slots.len() || bi >= inv.slots.len() {
                return Vec::new();
            }
            if !matches!(inv.slots.get(bi), Some(Some(ClientItem::Program(_)))) {
                return Vec::new();
            }
            let old_disc_prog = disc.slots[di].take();
            if let Some(ClientItem::Program(prog)) = inv.slots[bi].take() {
                disc.slots[di] = Some(prog);
            }
            inv.slots[bi] = old_disc_prog.map(ClientItem::Program);
            Vec::new()
        }
        // Program: Disc 槽位 -> 背包
        (SlotLoc::DiscSlot(di), SlotLoc::Bag(bi)) => {
            let Some(disc) = &mut inv.equipped_disc else {
                return Vec::new();
            };
            if di >= disc.slots.len() || bi >= inv.slots.len() {
                return Vec::new();
            }
            if let Some(prog) = disc.slots[di].take() {
                let old_bag = inv.slots[bi].take();
                inv.slots[bi] = Some(ClientItem::Program(prog));
                if let Some(ClientItem::Program(old_prog)) = old_bag {
                    disc.slots[di] = Some(old_prog);
                }
            }
            Vec::new()
        }
        // 背包格子互换
        (SlotLoc::Bag(a), SlotLoc::Bag(b)) => {
            if a < inv.slots.len() && b < inv.slots.len() {
                inv.slots.swap(a, b);
            }
            Vec::new()
        }
        // Disc 槽位互换
        (SlotLoc::DiscSlot(a), SlotLoc::DiscSlot(b)) => {
            if let Some(disc) = &mut inv.equipped_disc {
                if a < disc.slots.len() && b < disc.slots.len() {
                    disc.slots.swap(a, b);
                }
            }
            Vec::new()
        }
        // Disc: 背包 -> 装备
        (SlotLoc::Bag(bi), SlotLoc::EquippedDisc) => {
            if matches!(inv.slots.get(bi), Some(Some(ClientItem::Disc(_)))) {
                perform_disc_swap(inv, bi)
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

/// Disc 替换逻辑：将背包中的 Disc 替换为当前装备
/// 返回无法放入背包的被丢弃 Programs（用于生成到地面）
pub fn perform_disc_swap(
    inventory: &mut ClientInventory,
    new_disc_bag_index: usize,
) -> Vec<ClientProgram> {
    let Some(new_disc_item) = inventory.slots[new_disc_bag_index].take() else {
        return Vec::new();
    };
    let ClientItem::Disc(new_disc) = new_disc_item else {
        // 放回
        inventory.slots[new_disc_bag_index] = Some(new_disc_item);
        return Vec::new();
    };

    let old_disc = inventory.equipped_disc.take();
    let new_slot_count = new_disc.slots.len();

    // 从旧 Disc 收集 Program
    let old_programs: Vec<ClientProgram> = if let Some(old) = &old_disc {
        old.slots.iter().filter_map(|s| s.clone()).collect()
    } else {
        Vec::new()
    };

    // 构建新 Disc 槽位
    let mut new_slots: Vec<Option<ClientProgram>> = vec![None; new_slot_count];

    // 收集无法放入背包的 Programs
    let mut dropped_programs = Vec::new();

    // 按位置填充
    for (i, program) in old_programs.into_iter().enumerate() {
        if i < new_slot_count {
            new_slots[i] = Some(program);
        } else {
            // 多余 Program 尝试放入背包
            let mut placed = false;
            for slot in inventory.slots.iter_mut() {
                if slot.is_none() {
                    *slot = Some(ClientItem::Program(program.clone()));
                    placed = true;
                    break;
                }
            }
            if !placed {
                // 背包满，丢到地面
                dropped_programs.push(program);
            }
        }
    }

    let mut equipped = new_disc;
    equipped.slots = new_slots;
    inventory.equipped_disc = Some(equipped);

    // 旧 Disc 放回背包
    if let Some(old) = old_disc {
        let mut old_clean = old;
        old_clean.slots = vec![None; old_clean.slots.len()];
        inventory.slots[new_disc_bag_index] = Some(ClientItem::Disc(old_clean));
    }

    dropped_programs
}

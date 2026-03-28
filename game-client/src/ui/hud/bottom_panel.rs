//! 底部 HUD 面板：PC 两列布局（武器 | 背包），手机垂直堆叠
//!
//! 所有尺寸由 panel_height（屏幕高度 15%）推导，实现响应式缩放。
//! 布局使用 `compute_unified_layout` + `cell_rect` 手动定位，
//! 拖拽交互由 `slot` / `disc_info` / 排序按钮组件统一处理。
//! 双击背包物品：Disc->装备，Program->装入第一个空 Disc 槽位。
//! 拖出面板外 = 丢弃物品（反方向掉落）。

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use game_core::components::{ActiveBuffs, DiscEquipped, Position};

use crate::egui_setup::EguiColors;
use crate::input::LocalInput;
use crate::simulation::DropItemRequest;
use crate::sync::LocalPlayerMarker;
use crate::ui::drag::handle_drop;
use crate::ui::game::disc_info;
use crate::ui::game::slot::{slot, SlotContent, SlotLoc, SlotResponse};
use crate::ui::game::slot_grid::{cell_rect, compute_unified_layout, SLOT_GAP, SLOT_SIZE};
use crate::ui::{BackpackPager, ClientInventory, ClientItem, InputMode};

use super::HudLayout;

#[cfg(target_arch = "wasm32")]
use crate::js_bridge;

/// i18n helper
fn t(key: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        js_bridge::t(key)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        key.to_string()
    }
}

// ─── 底部面板主系统 ───

/// 底部面板主系统
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn bottom_panel_system(
    mut contexts: EguiContexts,
    hud_layout: Res<HudLayout>,
    disc_q: Query<&DiscEquipped, With<LocalPlayerMarker>>,
    buffs_q: Query<&ActiveBuffs, With<LocalPlayerMarker>>,
    local_pos_q: Query<&Position, With<LocalPlayerMarker>>,
    mut inventory: ResMut<ClientInventory>,
    _input_mode: Res<InputMode>,
    mut pager: ResMut<BackpackPager>,
    mut drop_writer: MessageWriter<DropItemRequest>,
    local_input: Res<LocalInput>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let lo = &*hud_layout;

    let mut drops: Vec<(SlotLoc, SlotLoc)> = Vec::new();
    let mut double_clicks: Vec<SlotLoc> = Vec::new();
    let mut sort_requested = false;
    let inv_ref = &*inventory;

    let panel_top_y = if hud_layout.is_mobile() {
        render_mobile_panel(
            ctx,
            &disc_q,
            inv_ref,
            &mut pager,
            &mut drops,
            &mut double_clicks,
            &mut sort_requested,
            lo,
        )
    } else {
        render_pc_panel(
            ctx,
            &hud_layout,
            &disc_q,
            &buffs_q,
            inv_ref,
            &mut pager,
            &mut drops,
            &mut double_clicks,
            &mut sort_requested,
            lo,
        )
    };

    // 排序背包：Disc 在前，Program 在中，空槽在后
    if sort_requested {
        sort_inventory(&mut inventory);
    }

    // 问题 5：面板上方区域作为丢弃 drop target
    // 在面板上方放置一个透明的全屏 hover 区域，捕获拖出面板外的 drop
    if let Ok(ctx2) = contexts.ctx_mut() {
        let discard_rect =
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(lo.screen_w, panel_top_y));
        let discard_resp = egui::Area::new(egui::Id::new("discard_drop_zone"))
            .fixed_pos(discard_rect.min)
            .order(egui::Order::Background)
            .interactable(true)
            .show(ctx2, |ui| {
                let (_, resp) = ui.allocate_exact_size(discard_rect.size(), egui::Sense::hover());
                resp.dnd_release_payload::<SlotLoc>().map(|src| *src)
            });
        if let Some(src) = discard_resp.inner {
            drops.push((src, SlotLoc::Trash));
        }
    }

    // 双击快捷操作转为 drops
    for loc in double_clicks {
        let item = match loc {
            SlotLoc::Bag(i) => inventory.slots.get(i).and_then(|s| s.as_ref()),
            _ => None,
        };
        if let Some(it) = item {
            match it {
                ClientItem::Disc(_) => {
                    drops.push((loc, SlotLoc::EquippedDisc));
                }
                ClientItem::Program(_) => {
                    if let Some(disc) = &inventory.equipped_disc {
                        if let Some(slot_idx) = disc.slots.iter().position(|s| s.is_none()) {
                            drops.push((loc, SlotLoc::DiscSlot(slot_idx)));
                        }
                    }
                }
            }
        }
    }

    // 获取玩家位置用于丢弃物品
    let player_pos = local_pos_q.single().map(|p| Vec2::new(p.x, p.y)).ok();

    for (from, to) in drops {
        if to == SlotLoc::Trash {
            let Some(pos) = player_pos else { continue };
            // 丢弃方向 = 玩家移动方向的反方向
            let move_dir = Vec2::new(local_input.move_x, local_input.move_y);
            let drop_offset = if move_dir.length_squared() > 0.01 {
                -move_dir.normalize() * 48.0
            } else {
                Vec2::new(0.0, -48.0) // 默认向下丢
            };
            let drop_pos = pos + drop_offset;
            match from {
                SlotLoc::Bag(idx) => {
                    if inventory.slots.get(idx).is_some_and(|s| s.is_some()) {
                        let item = inventory.slots[idx].take();
                        if let Some(client_item) = item {
                            let prog = match &client_item {
                                ClientItem::Program(p) => Some(p.clone().into()),
                                ClientItem::Disc(_d) => None,
                            };
                            drop_writer.write(DropItemRequest {
                                bag_index: Some(idx),
                                disc_slot_index: None,
                                item: prog,
                                pos: drop_pos,
                            });
                        }
                    }
                }
                SlotLoc::DiscSlot(idx) => {
                    if let Some(disc) = &mut inventory.equipped_disc {
                        if idx < disc.slots.len() && disc.slots[idx].is_some() {
                            let prog = disc.slots[idx].take().map(|p| p.into());
                            drop_writer.write(DropItemRequest {
                                bag_index: None,
                                disc_slot_index: Some(idx),
                                item: prog,
                                pos: drop_pos,
                            });
                        }
                    }
                }
                _ => {}
            }
        } else {
            let dropped = handle_drop(&mut inventory, from, to);
            for prog in dropped {
                if let Some(pos) = player_pos {
                    drop_writer.write(DropItemRequest {
                        bag_index: None,
                        disc_slot_index: None,
                        item: Some(prog.into()),
                        pos,
                    });
                }
            }
        }
    }
}

/// 排序背包：Disc 在前，Program 在中，空槽在后
fn sort_inventory(inventory: &mut ClientInventory) {
    inventory.slots.sort_by(|a, b| match (a, b) {
        (Some(ClientItem::Disc(_)), Some(ClientItem::Disc(_))) => std::cmp::Ordering::Equal,
        (Some(ClientItem::Disc(_)), _) => std::cmp::Ordering::Less,
        (_, Some(ClientItem::Disc(_))) => std::cmp::Ordering::Greater,
        (Some(ClientItem::Program(_)), Some(ClientItem::Program(_))) => std::cmp::Ordering::Equal,
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}

/// 收集槽位交互响应（drop 和双击）
fn collect_response(
    resp: SlotResponse,
    target_loc: SlotLoc,
    drops: &mut Vec<(SlotLoc, SlotLoc)>,
    double_clicks: &mut Vec<SlotLoc>,
) {
    if let Some(source) = resp.drop_source {
        drops.push((source, target_loc));
    }
    if resp.double_clicked {
        double_clicks.push(target_loc);
    }
}

// ═══════════════════════════════════════════════════
// PC 底部面板
// ═══════════════════════════════════════════════════

/// 返回面板顶部 y 坐标（用于判断拖出面板外）
#[allow(clippy::too_many_arguments)]
fn render_pc_panel(
    ctx: &egui::Context,
    hud: &HudLayout,
    disc_q: &Query<&DiscEquipped, With<LocalPlayerMarker>>,
    _buffs_q: &Query<&ActiveBuffs, With<LocalPlayerMarker>>,
    inventory: &ClientInventory,
    pager: &mut BackpackPager,
    drops: &mut Vec<(SlotLoc, SlotLoc)>,
    double_clicks: &mut Vec<SlotLoc>,
    sort_requested: &mut bool,
    lo: &HudLayout,
) -> f32 {
    let panel_h = hud.panel_height;
    let sw = lo.screen_w;

    egui::TopBottomPanel::bottom("hud_bottom")
        .exact_height(panel_h)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            let panel_w = ui.available_width();
            let content_w = panel_w.min(1440.0);
            let margin_left = (panel_w - content_w) / 2.0;
            let bg_origin = ui.min_rect().min + egui::vec2(margin_left, 0.0);

            // 内容区背景 + 顶部分割线
            let content_rect = egui::Rect::from_min_size(bg_origin, egui::vec2(content_w, panel_h));
            let painter = ui.painter();
            painter.rect_filled(content_rect, 4.0, EguiColors::PANEL_BG);
            painter.line_segment(
                [content_rect.left_top(), content_rect.right_top()],
                egui::Stroke::new(1.0, EguiColors::BORDER_SUBTLE),
            );

            // 内边距：grid 区域 = content 减去四边 padding
            let pad = SLOT_GAP;
            let inner_w = content_w - pad * 2.0;
            let inner_h = panel_h - pad * 2.0;
            let origin = bg_origin + egui::vec2(pad, pad);

            // 统一网格：cols/rows/gap 全部从 compute_unified_layout 取
            let (total_cols, total_rows, gap) =
                compute_unified_layout(inner_w, inner_h, SLOT_SIZE, SLOT_GAP);

            // 按 4:6 从 total_cols 分配武器/背包列数
            let weapon_cols = (total_cols * 4 / 10).max(2);
            let backpack_cols = total_cols.saturating_sub(weapon_cols);
            let slot_rows = total_rows.saturating_sub(1);

            // DEBUG
            #[cfg(target_arch = "wasm32")]
            {
                use std::sync::atomic::{AtomicU64, Ordering};
                static LAST: AtomicU64 = AtomicU64::new(0);
                let now = web_sys::js_sys::Date::now() as u64;
                if now - LAST.load(Ordering::Relaxed) > 2000 {
                    LAST.store(now, Ordering::Relaxed);
                    web_sys::console::log_1(
                        &format!(
                            "[GRID] {:.0}x{:.0} cols={}(w{}+b{}) rows={} sr={} gap={:.1}",
                            content_w,
                            panel_h,
                            total_cols,
                            weapon_cols,
                            backpack_cols,
                            total_rows,
                            slot_rows,
                            gap,
                        )
                        .into(),
                    );
                }
            }

            // ─── Row 0: Header ───

            // [0,0] Disc 图标（EquippedDisc drop target）
            let icon_rect = cell_rect(origin, 0, 0, 1, 1, SLOT_SIZE, gap);
            ui.scope_builder(egui::UiBuilder::new().max_rect(icon_rect), |ui| {
                let disc = disc_q.single().ok();
                let font_m = (SLOT_SIZE * 0.3).clamp(11.0, 18.0);
                let font_s = (SLOT_SIZE * 0.25).clamp(7.0, 13.0);
                if let Some(src) =
                    disc_info::disc_info(ui, disc, SLOT_SIZE, font_m, font_s, SLOT_GAP)
                {
                    drops.push((src, SlotLoc::EquippedDisc));
                }
            });

            // [1..weapon_cols-1, 0] 武器品质+属性文字
            if weapon_cols > 1 {
                let info_rect = cell_rect(origin, 1, 0, weapon_cols - 1, 1, SLOT_SIZE, gap);
                ui.scope_builder(egui::UiBuilder::new().max_rect(info_rect), |ui| {
                    if let Ok(disc) = disc_q.single() {
                        let font_s = (SLOT_SIZE * 0.2).clamp(8.0, 14.0);
                        ui.horizontal_centered(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            crate::ui::game::stat_label(
                                ui,
                                &t("ui.hud.stat_dmg"),
                                EguiColors::COLOR_ENEMY,
                                &format!("{:.0}", 1.0 / disc.disc.base_fire_rate),
                                font_s,
                                gap,
                            );
                            crate::ui::game::stat_label(
                                ui,
                                &t("ui.hud.stat_acc"),
                                EguiColors::CYAN,
                                &format!("{:.0}\u{00B0}", disc.disc.base_spread.to_degrees()),
                                font_s,
                                gap,
                            );
                        });
                    }
                });
            }

            // [weapon_cols, 0] 排序按钮
            if backpack_cols > 0 {
                let sort_rect = cell_rect(origin, weapon_cols, 0, 1, 1, SLOT_SIZE, gap);
                ui.scope_builder(egui::UiBuilder::new().max_rect(sort_rect), |ui| {
                    let font_l = (SLOT_SIZE * 0.3).clamp(11.0, 18.0);
                    if sort_button(ui, SLOT_SIZE, font_l) {
                        *sort_requested = true;
                    }
                });
            }

            // [weapon_cols+1..total_cols-1, 0] 背包标题 + 翻页
            if backpack_cols > 1 {
                let bp_header_rect = cell_rect(
                    origin,
                    weapon_cols + 1,
                    0,
                    backpack_cols - 1,
                    1,
                    SLOT_SIZE,
                    gap,
                );
                ui.scope_builder(egui::UiBuilder::new().max_rect(bp_header_rect), |ui| {
                    let font_s = (SLOT_SIZE * 0.2).clamp(8.0, 14.0);
                    let bp_per_page = if backpack_cols > 0 && slot_rows > 0 {
                        backpack_cols * slot_rows
                    } else {
                        1
                    };
                    let total_pages = if bp_per_page > 0 {
                        ClientInventory::MAX_SLOTS.div_ceil(bp_per_page)
                    } else {
                        1
                    };
                    if pager.page >= total_pages {
                        pager.page = total_pages.saturating_sub(1);
                    }

                    ui.horizontal_centered(|ui| {
                        ui.label(
                            egui::RichText::new(t("ui.hud.backpack"))
                                .size(font_s)
                                .color(EguiColors::TEXT_TERTIARY)
                                .strong(),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if total_pages > 1 {
                                render_pager_buttons(ui, pager, total_pages, font_s);
                            }
                        });
                    });
                });
            }

            // ─── Row 1+: 槽位 ───

            // 左侧 Disc Program 槽位
            if let Some(equipped) = &inventory.equipped_disc {
                let disc_slot_count = equipped.slots.len();
                for row in 0..slot_rows {
                    for col in 0..weapon_cols {
                        let idx = row * weapon_cols + col;
                        let rect = cell_rect(origin, col, row + 1, 1, 1, SLOT_SIZE, gap);
                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            if idx < disc_slot_count {
                                let content = SlotContent::DiscProgram(
                                    equipped.slots.get(idx).and_then(|s| s.as_ref()),
                                );
                                let loc = SlotLoc::DiscSlot(idx);
                                let id = egui::Id::new(("disc_slot_pc", idx));
                                let resp = slot(ui, id, loc, &content, SLOT_SIZE, true);
                                collect_response(resp, loc, drops, double_clicks);
                            } else {
                                // 超出 disc 槽位数：空占位
                                let id = egui::Id::new(("disc_empty_pc", idx));
                                let _ = slot(
                                    ui,
                                    id,
                                    SlotLoc::Bag(0),
                                    &SlotContent::Locked,
                                    SLOT_SIZE,
                                    false,
                                );
                            }
                        });
                    }
                }
            }

            // 右侧背包槽位（翻页）
            let bp_per_page = if backpack_cols > 0 && slot_rows > 0 {
                backpack_cols * slot_rows
            } else {
                1
            };
            let total = ClientInventory::MAX_SLOTS;
            let unlocked = inventory.max_slots;
            let start = pager.page * bp_per_page;

            for row in 0..slot_rows {
                for col in 0..backpack_cols {
                    let local_idx = row * backpack_cols + col;
                    let global_idx = start + local_idx;
                    let rect = cell_rect(origin, weapon_cols + col, row + 1, 1, 1, SLOT_SIZE, gap);
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                        if global_idx >= total || local_idx >= bp_per_page {
                            let id = egui::Id::new(("bp_slot_pc", global_idx));
                            let _ = slot(
                                ui,
                                id,
                                SlotLoc::Bag(global_idx),
                                &SlotContent::Locked,
                                SLOT_SIZE,
                                false,
                            );
                        } else if global_idx >= unlocked {
                            let id = egui::Id::new(("bp_slot_pc", global_idx));
                            let resp = slot(
                                ui,
                                id,
                                SlotLoc::Bag(global_idx),
                                &SlotContent::Locked,
                                SLOT_SIZE,
                                false,
                            );
                            collect_response(resp, SlotLoc::Bag(global_idx), drops, double_clicks);
                        } else {
                            let item = inventory.slots.get(global_idx).and_then(|s| s.as_ref());
                            let content = match item {
                                Some(it) => SlotContent::BagItem(it),
                                None => SlotContent::Empty,
                            };
                            let loc = SlotLoc::Bag(global_idx);
                            let id = egui::Id::new(("bp_slot_pc", global_idx));
                            let resp = slot(ui, id, loc, &content, SLOT_SIZE, true);
                            collect_response(resp, loc, drops, double_clicks);
                        }
                    });
                }
            }

            // 背包区域 fallback：拖到空白处放入第一个空槽
            let bp_area = egui::Rect::from_min_size(
                origin + egui::vec2(weapon_cols as f32 * (SLOT_SIZE + gap), SLOT_SIZE + gap),
                egui::vec2(
                    backpack_cols as f32 * (SLOT_SIZE + gap),
                    slot_rows as f32 * (SLOT_SIZE + gap),
                ),
            );
            let bag_resp = ui.interact(
                bp_area,
                egui::Id::new("backpack_fallback"),
                egui::Sense::hover(),
            );
            if let Some(src) = bag_resp.dnd_release_payload::<SlotLoc>() {
                if let Some(empty_idx) = inventory.slots.iter().position(|s| s.is_none()) {
                    drops.push((*src, SlotLoc::Bag(empty_idx)));
                }
            }

            // Divider：竖线在武器和背包区域之间的 gap 中线
            let div_x = origin.x + weapon_cols as f32 * (SLOT_SIZE + gap) - gap / 2.0;
            ui.painter().line_segment(
                [
                    egui::pos2(div_x, origin.y),
                    egui::pos2(div_x, origin.y + panel_h),
                ],
                egui::Stroke::new(1.0, EguiColors::BORDER_SUBTLE),
            );

            // 消除 sw 未使用警告
            let _ = sw;
        });

    // 面板顶部 y = 屏幕高度 - 面板高度
    lo.screen_h - panel_h
}

// ═══════════════════════════════════════════════════
// Mobile 底部面板
// ═══════════════════════════════════════════════════

/// 返回面板顶部 y 坐标
#[allow(clippy::too_many_arguments)]
fn render_mobile_panel(
    ctx: &egui::Context,
    disc_q: &Query<&DiscEquipped, With<LocalPlayerMarker>>,
    inventory: &ClientInventory,
    pager: &mut BackpackPager,
    drops: &mut Vec<(SlotLoc, SlotLoc)>,
    double_clicks: &mut Vec<SlotLoc>,
    sort_requested: &mut bool,
    lo: &HudLayout,
) -> f32 {
    let panel_h = lo.panel_height;

    egui::TopBottomPanel::bottom("hud_bottom_mobile")
        .exact_height(panel_h)
        .resizable(false)
        .frame(egui::Frame::NONE.fill(EguiColors::PANEL_BG))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            let w = ui.available_width();
            let origin = ui.min_rect().min;

            // 顶部分割线
            ui.painter().line_segment(
                [origin, egui::pos2(origin.x + w, origin.y)],
                egui::Stroke::new(1.0, EguiColors::BORDER_SUBTLE),
            );

            // 统一网格
            let (cols, total_rows, gap) = compute_unified_layout(w, panel_h, SLOT_SIZE, SLOT_GAP);

            // 行分配：row 0 = header, row 1 = disc 槽位, 水平分割线, row 2+ = 背包
            let disc_slot_count = inventory
                .equipped_disc
                .as_ref()
                .map_or(0, |d| d.slots.len());
            let disc_rows = if disc_slot_count > 0 && cols > 0 {
                disc_slot_count.div_ceil(cols)
            } else {
                0
            };
            let bp_rows = total_rows.saturating_sub(1 + disc_rows);

            // ─── Row 0: Header（全宽） ───
            // [0,0] Disc 图标
            let icon_rect = cell_rect(origin, 0, 0, 1, 1, SLOT_SIZE, gap);
            ui.scope_builder(egui::UiBuilder::new().max_rect(icon_rect), |ui| {
                let disc = disc_q.single().ok();
                let font_m = (SLOT_SIZE * 0.3).clamp(11.0, 18.0);
                let font_s = (SLOT_SIZE * 0.25).clamp(7.0, 13.0);
                if let Some(src) =
                    disc_info::disc_info(ui, disc, SLOT_SIZE, font_m, font_s, SLOT_GAP)
                {
                    drops.push((src, SlotLoc::EquippedDisc));
                }
            });

            // [1,0] 排序按钮
            if cols > 1 {
                let sort_rect = cell_rect(origin, 1, 0, 1, 1, SLOT_SIZE, gap);
                ui.scope_builder(egui::UiBuilder::new().max_rect(sort_rect), |ui| {
                    let font_l = (SLOT_SIZE * 0.3).clamp(11.0, 18.0);
                    if sort_button(ui, SLOT_SIZE, font_l) {
                        *sort_requested = true;
                    }
                });
            }

            // [2..cols-1, 0] 背包标题 + 翻页
            if cols > 2 {
                let header_span = cols - 2;
                let bp_header_rect = cell_rect(origin, 2, 0, header_span, 1, SLOT_SIZE, gap);
                ui.scope_builder(egui::UiBuilder::new().max_rect(bp_header_rect), |ui| {
                    let font_s = (SLOT_SIZE * 0.2).clamp(8.0, 14.0);
                    let bp_per_page = if cols > 0 && bp_rows > 0 {
                        cols * bp_rows
                    } else {
                        1
                    };
                    let total_pages = if bp_per_page > 0 {
                        ClientInventory::MAX_SLOTS.div_ceil(bp_per_page)
                    } else {
                        1
                    };
                    if pager.page >= total_pages {
                        pager.page = total_pages.saturating_sub(1);
                    }

                    ui.horizontal_centered(|ui| {
                        ui.label(
                            egui::RichText::new(t("ui.hud.backpack"))
                                .size(font_s)
                                .color(EguiColors::TEXT_TERTIARY)
                                .strong(),
                        );
                        if total_pages > 1 {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    render_pager_buttons(ui, pager, total_pages, font_s);
                                },
                            );
                        }
                    });
                });
            }

            // ─── Row 1..1+disc_rows: Disc 槽位（全宽） ───
            if let Some(equipped) = &inventory.equipped_disc {
                for row in 0..disc_rows {
                    for col in 0..cols {
                        let idx = row * cols + col;
                        if idx >= disc_slot_count {
                            break;
                        }
                        let rect = cell_rect(origin, col, 1 + row, 1, 1, SLOT_SIZE, gap);
                        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            let content = SlotContent::DiscProgram(
                                equipped.slots.get(idx).and_then(|s| s.as_ref()),
                            );
                            let loc = SlotLoc::DiscSlot(idx);
                            let id = egui::Id::new(("disc_slot_mob", idx));
                            let resp = slot(ui, id, loc, &content, SLOT_SIZE, true);
                            collect_response(resp, loc, drops, double_clicks);
                        });
                    }
                }
            }

            // Divider：水平线在 disc 行和背包行之间
            let div_y = origin.y + (1 + disc_rows) as f32 * (SLOT_SIZE + gap) - gap / 2.0;
            ui.painter().line_segment(
                [egui::pos2(origin.x, div_y), egui::pos2(origin.x + w, div_y)],
                egui::Stroke::new(1.0, EguiColors::BORDER_SUBTLE),
            );

            // ─── Row (1+disc_rows)..total_rows: 背包槽位（全宽，翻页） ───
            let bp_per_page = if cols > 0 && bp_rows > 0 {
                cols * bp_rows
            } else {
                1
            };
            let total = ClientInventory::MAX_SLOTS;
            let unlocked = inventory.max_slots;
            let start = pager.page * bp_per_page;
            let bp_start_row = 1 + disc_rows;

            for row in 0..bp_rows {
                for col in 0..cols {
                    let local_idx = row * cols + col;
                    let global_idx = start + local_idx;
                    let rect = cell_rect(origin, col, bp_start_row + row, 1, 1, SLOT_SIZE, gap);
                    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                        if global_idx >= total || local_idx >= bp_per_page {
                            let id = egui::Id::new(("bp_slot_mob", global_idx));
                            let _ = slot(
                                ui,
                                id,
                                SlotLoc::Bag(global_idx),
                                &SlotContent::Locked,
                                SLOT_SIZE,
                                false,
                            );
                        } else if global_idx >= unlocked {
                            let id = egui::Id::new(("bp_slot_mob", global_idx));
                            let resp = slot(
                                ui,
                                id,
                                SlotLoc::Bag(global_idx),
                                &SlotContent::Locked,
                                SLOT_SIZE,
                                false,
                            );
                            collect_response(resp, SlotLoc::Bag(global_idx), drops, double_clicks);
                        } else {
                            let item = inventory.slots.get(global_idx).and_then(|s| s.as_ref());
                            let content = match item {
                                Some(it) => SlotContent::BagItem(it),
                                None => SlotContent::Empty,
                            };
                            let loc = SlotLoc::Bag(global_idx);
                            let id = egui::Id::new(("bp_slot_mob", global_idx));
                            let resp = slot(ui, id, loc, &content, SLOT_SIZE, true);
                            collect_response(resp, loc, drops, double_clicks);
                        }
                    });
                }
            }
        });

    // 面板顶部 y = 屏幕高度 - 面板高度
    lo.screen_h - panel_h
}

// ═══════════════════════════════════════════════════
// 排序按钮
// ═══════════════════════════════════════════════════

/// 排序按钮：点击后排序背包
/// 返回 true 表示按钮被点击
fn sort_button(ui: &mut egui::Ui, size: f32, font_size: f32) -> bool {
    let slot_vec = egui::vec2(size, size);
    let (rect, resp) = ui.allocate_exact_size(slot_vec, egui::Sense::click());

    let is_hover = resp.hovered();
    let border_color = if is_hover {
        EguiColors::CYAN
    } else {
        EguiColors::BORDER_SUBTLE
    };
    let icon_color = if is_hover {
        EguiColors::CYAN
    } else {
        EguiColors::TEXT_TERTIARY
    };

    let p = ui.painter();
    p.rect_filled(rect, 4.0, EguiColors::BG_INSET);
    p.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, border_color),
        egui::StrokeKind::Inside,
    );
    // 排序图标 ⇅
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "\u{21C5}",
        egui::FontId::proportional(font_size * 0.7),
        icon_color,
    );

    resp.clicked()
}

// ═══════════════════════════════════════════════════
// 翻页按钮
// ═══════════════════════════════════════════════════

/// 渲染 < page/total > 翻页按钮（在 right_to_left 布局中调用）
fn render_pager_buttons(
    ui: &mut egui::Ui,
    pager: &mut BackpackPager,
    total_pages: usize,
    font_s: f32,
) {
    let btn_size = egui::vec2(font_s * 1.5, font_s * 1.5);
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new(">")
                    .size(font_s)
                    .color(EguiColors::CYAN),
            )
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE)
            .min_size(btn_size),
        )
        .clicked()
        && pager.page + 1 < total_pages
    {
        pager.page += 1;
    }
    ui.label(
        egui::RichText::new(format!("{}/{}", pager.page + 1, total_pages))
            .size(font_s)
            .color(EguiColors::TEXT_SECONDARY),
    );
    if ui
        .add(
            egui::Button::new(
                egui::RichText::new("<")
                    .size(font_s)
                    .color(EguiColors::CYAN),
            )
            .fill(egui::Color32::TRANSPARENT)
            .stroke(egui::Stroke::NONE)
            .min_size(btn_size),
        )
        .clicked()
        && pager.page > 0
    {
        pager.page -= 1;
    }
}

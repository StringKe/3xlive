//! HUD 插件：底部面板、玩家信息浮层、Buff 浮层、通知

pub(crate) mod bottom_panel;
pub(crate) mod buff_bar;
mod notification;
mod player_info;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use game_core::components::ActiveBuffs;

use crate::egui_setup::EguiColors;
use crate::simulation::run_lifecycle::RunState;
use crate::sync::LocalPlayerMarker;
use crate::ui::components::{rem, t};
use crate::ui::game::slot::{slot, SlotContent, SlotLoc};
use crate::GameState;

use player_info::player_info_overlay_system;

// ─── 布局常量 ───

/// 内容区域最大宽度
const MAX_CONTENT_W: f32 = 1440.0;

/// 面板高度占屏幕高度的比例
/// 25% 可容纳 header + 2 行 72px 槽位 (72*3 + 12*2 = 240px at 1080p = 270px)
const PANEL_HEIGHT_RATIO: f32 = 0.25;
/// 面板最小高度（像素）—— 至少容纳 header + 1 行槽位
const PANEL_MIN_HEIGHT: f32 = 160.0;

// ─── HudLayout Resource ───

/// 全局 HUD 布局参数，每帧由 hud_layout_update_system 更新
/// 包含屏幕尺寸、内容框、面板高度和响应式 UI 尺寸
#[derive(Resource, Default, Clone, Debug)]
pub struct HudLayout {
    // 屏幕与内容框
    pub screen_w: f32,
    pub screen_h: f32,
    pub content_w: f32,
    pub margin: f32,
    pub panel_height: f32,
}

impl HudLayout {
    pub fn content_left(&self) -> f32 {
        self.margin
    }

    pub fn content_right(&self) -> f32 {
        self.margin + self.content_w
    }

    pub fn is_mobile(&self) -> bool {
        self.screen_w < 900.0
    }
}

/// 每帧更新 HudLayout（无 GameState 守卫，始终运行）
fn hud_layout_update_system(mut contexts: EguiContexts, mut hud: ResMut<HudLayout>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let screen = ctx.content_rect();
    let sw = screen.width();
    let sh = screen.height();
    let cw = sw.min(MAX_CONTENT_W);
    let margin = (sw - cw) * 0.5;
    let ph = (sh * PANEL_HEIGHT_RATIO).max(PANEL_MIN_HEIGHT);

    hud.screen_w = sw;
    hud.screen_h = sh;
    hud.content_w = cw;
    hud.margin = margin;
    hud.panel_height = ph;
}

// ═══════════════════════════════════════════════════
// Buff 浮层（右上角）
// ═══════════════════════════════════════════════════

/// Buff 浮层：水平排列当前激活的 buff 图标（使用 slot 组件）
fn buff_overlay_system(
    mut contexts: EguiContexts,
    hud: Res<HudLayout>,
    buffs_q: Query<&ActiveBuffs, With<LocalPlayerMarker>>,
) {
    let active_buffs: Vec<_> = buffs_q
        .single()
        .map(|b| b.buffs.clone())
        .unwrap_or_default();

    if active_buffs.is_empty() {
        return;
    }

    let sw = hud.screen_w;
    let sb = rem(sw, 0.5);
    let buff_size = (sb * 2.0).clamp(14.0, 24.0);

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // 预估宽度：buff 数 * (buff_size + gap) + margin
    let estimated_w = active_buffs.len() as f32 * (buff_size + sb) + 16.0;
    let pos = egui::pos2(hud.content_right() - estimated_w, 8.0);

    // 使用固定位置的 Area（overlay 组件的 TopRight 锚点会右对齐，此处保持原始定位逻辑）
    egui::Area::new(egui::Id::new("hud_buff_overlay"))
        .fixed_pos(pos)
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(EguiColors::PANEL_BG)
                .stroke(egui::Stroke::new(1.0, EguiColors::CYAN_FAINT))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = sb;
                        for (i, buff) in active_buffs.iter().enumerate() {
                            let buff_col = buff_bar::buff_color(buff.buff_type);
                            let name = t(buff_bar::buff_name_key(&buff.buff_type));
                            let content = SlotContent::Buff {
                                color: buff_col,
                                name: name.clone(),
                            };
                            let id = egui::Id::new(("buff_slot", i));
                            let resp = slot(
                                ui,
                                id,
                                SlotLoc::Bag(0), // Buff 不参与拖拽，loc 占位
                                &content,
                                buff_size,
                                false,
                            );
                            // Tooltip：slot 组件不为 Buff 类型生成 tooltip，手动处理
                            let _ = resp;
                            // 检测 hover 状态用 ui.ctx().pointer_hover_pos
                            // Buff slot 无拖拽交互，直接用最后分配的 rect 检测
                        }
                    });
                });
        });
}

// ─── Plugin ───

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudLayout>();

        // 始终运行：布局更新（读取 EguiContexts，需在 EguiPrimaryContextPass 中运行）
        app.add_systems(bevy_egui::EguiPrimaryContextPass, hud_layout_update_system);

        // Playing 状态下运行的 egui 渲染系统
        // bottom_panel 在结算状态下隐藏，避免和结算弹窗重叠
        app.add_systems(
            bevy_egui::EguiPrimaryContextPass,
            bottom_panel::bottom_panel_system
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(RunState::Playing)),
        );
        app.add_systems(
            bevy_egui::EguiPrimaryContextPass,
            (
                notification::notification_render_system,
                player_info_overlay_system,
                buff_overlay_system,
            )
                .run_if(in_state(GameState::Playing)),
        );

        // Playing 状态下运行的纯 ECS 系统（不使用 egui）
        app.add_systems(
            Update,
            (
                notification::notification_handler,
                notification::notification_fade,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

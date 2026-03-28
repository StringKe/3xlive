//! 虚拟摇杆：触屏设备的移动输入控件

use bevy::input::touch::{TouchInput, TouchPhase};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::egui_setup::EguiColors;
use crate::input::{LocalInput, TouchActive};
use crate::GameState;

use super::InputMode;

// ─── 常量 ───

/// 摇杆底座半径（像素）
const BASE_RADIUS: f32 = 64.0;
/// 拇指圆半径（像素）
const THUMB_RADIUS: f32 = 24.0;

// ─── 资源 ───

/// 虚拟摇杆状态
#[derive(Resource, Default, Debug)]
pub struct VirtualJoystickState {
    /// 当前追踪的触摸 ID
    pub tracking_id: Option<u64>,
    /// 相对底座中心的偏移（像素，游戏坐标系：Y 向上）
    pub offset: bevy::math::Vec2,
}

// ─── 系统 ───

/// 处理触摸事件，驱动摇杆输入
fn joystick_touch_system(
    mut touch_events: MessageReader<TouchInput>,
    mut joystick: ResMut<VirtualJoystickState>,
    mut local_input: ResMut<LocalInput>,
    touch_active: Res<TouchActive>,
    input_mode: Res<InputMode>,
    hud_layout: Res<crate::ui::hud::HudLayout>,
    mut contexts: bevy_egui::EguiContexts,
) {
    // 非触屏模式或 UI 模式下跳过
    if !touch_active.0 {
        return;
    }
    if matches!(*input_mode, InputMode::UiMode { .. }) {
        return;
    }
    // egui 正在消费指针输入时跳过（防止与 UI 拖拽冲突）
    if contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.wants_pointer_input())
    {
        return;
    }

    // 底座中心：对齐 content frame 左下角（屏幕坐标系：Y 向下）
    let base_center = bevy::math::Vec2::new(
        hud_layout.content_left() + BASE_RADIUS + 32.0,
        hud_layout.screen_h - hud_layout.panel_height - BASE_RADIUS - 16.0,
    );

    for event in touch_events.read() {
        match event.phase {
            TouchPhase::Started => {
                // 已在追踪则忽略新触点
                if joystick.tracking_id.is_some() {
                    continue;
                }
                let dist = event.position.distance(base_center);
                if dist < BASE_RADIUS {
                    joystick.tracking_id = Some(event.id);
                    let raw_offset = event.position - base_center;
                    let clamped = clamp_offset(raw_offset);
                    // Y 轴取反：屏幕 Y 向下 → 游戏 Y 向上
                    joystick.offset = bevy::math::Vec2::new(clamped.x, -clamped.y);
                    write_input(&joystick, &mut local_input);
                }
            }
            TouchPhase::Moved => {
                if joystick.tracking_id != Some(event.id) {
                    continue;
                }
                let raw_offset = event.position - base_center;
                let clamped = clamp_offset(raw_offset);
                joystick.offset = bevy::math::Vec2::new(clamped.x, -clamped.y);
                write_input(&joystick, &mut local_input);
            }
            TouchPhase::Ended | TouchPhase::Canceled => {
                if joystick.tracking_id != Some(event.id) {
                    continue;
                }
                joystick.tracking_id = None;
                joystick.offset = bevy::math::Vec2::ZERO;
                local_input.move_x = 0.0;
                local_input.move_y = 0.0;
            }
        }
    }
}

/// 将偏移向量 clamp 到底座半径内
fn clamp_offset(raw: bevy::math::Vec2) -> bevy::math::Vec2 {
    let len = raw.length();
    if len > BASE_RADIUS {
        raw * (BASE_RADIUS / len)
    } else {
        raw
    }
}

/// 将摇杆偏移归一化后写入 LocalInput
fn write_input(joystick: &VirtualJoystickState, input: &mut LocalInput) {
    let len = joystick.offset.length();
    if len > 0.0 {
        input.move_x = joystick.offset.x / BASE_RADIUS;
        input.move_y = joystick.offset.y / BASE_RADIUS;
    } else {
        input.move_x = 0.0;
        input.move_y = 0.0;
    }
}

/// 用 egui layer_painter 绘制摇杆视觉反馈
fn joystick_render_system(
    mut contexts: EguiContexts,
    joystick: Res<VirtualJoystickState>,
    touch_active: Res<TouchActive>,
    hud_layout: Res<crate::ui::hud::HudLayout>,
) {
    if !touch_active.0 {
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Middle,
        egui::Id::new("joystick"),
    ));

    // 底座位置对齐 content frame 左下角
    let base_x = hud_layout.content_left() + BASE_RADIUS + 32.0;
    let base_y = hud_layout.screen_h - hud_layout.panel_height - BASE_RADIUS - 16.0;
    let center = egui::pos2(base_x, base_y);

    // 底座填充
    painter.circle_filled(center, BASE_RADIUS, EguiColors::CYAN_GHOST);
    // 底座描边
    painter.circle_stroke(
        center,
        BASE_RADIUS,
        egui::Stroke::new(2.0, EguiColors::CYAN_FAINT),
    );

    // 拇指圆：offset 的 Y 轴已取反（游戏坐标），渲染时再反回屏幕坐标
    let thumb = egui::vec2(joystick.offset.x, -joystick.offset.y);
    painter.circle_filled(center + thumb, THUMB_RADIUS, EguiColors::CYAN);
}

// ─── 插件 ───

pub struct VirtualJoystickPlugin;

impl Plugin for VirtualJoystickPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VirtualJoystickState>()
            // egui 系统：触控和渲染都访问 EguiContexts，需在 EguiPrimaryContextPass 中运行
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                (joystick_touch_system, joystick_render_system)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

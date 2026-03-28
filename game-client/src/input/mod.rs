use bevy::input::touch::TouchInput;
use bevy::prelude::*;
use bevy::window::CursorMoved;
use bevy_egui::EguiContexts;

use crate::GameState;

// ─── 资源 ───

/// 本地输入状态，每帧由 input system 写入
#[derive(Resource, Default, Debug, Clone)]
pub struct LocalInput {
    pub move_x: f32,
    pub move_y: f32,
}

/// 触控激活状态：检测当前输入方式是触屏还是鼠标
#[derive(Resource, Default, Debug, Clone)]
pub struct TouchActive(pub bool);

// ─── 插件 ───

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LocalInput>()
            .init_resource::<TouchActive>()
            // egui 系统：检查 ctx.wants_keyboard_input()，需在 EguiPrimaryContextPass 中运行
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                keyboard_input_system.run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, touch_detect_system);
    }
}

// ─── 键盘输入 ───

fn keyboard_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut input: ResMut<LocalInput>,
    mut contexts: EguiContexts,
) {
    // egui 捕获键盘时跳过游戏输入
    if contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.wants_keyboard_input())
    {
        input.move_x = 0.0;
        input.move_y = 0.0;
        return;
    }

    let mut mx = 0.0_f32;
    let mut my = 0.0_f32;

    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        my += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        my -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        mx -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        mx += 1.0;
    }

    // 归一化
    let len = (mx * mx + my * my).sqrt();
    if len > 0.0 {
        mx /= len;
        my /= len;
    }

    input.move_x = mx;
    input.move_y = my;
}

// ─── 触控检测 ───

/// 检测当前输入方式：TouchInput 事件 → 触屏模式，CursorMoved 事件 → 鼠标模式
/// 触屏模式具有粘滞性：一旦检测到触摸，只有真实鼠标移动才切回鼠标模式
fn touch_detect_system(
    mut touch_events: MessageReader<TouchInput>,
    mut cursor_events: MessageReader<CursorMoved>,
    mut touch_active: ResMut<TouchActive>,
) {
    let has_touch = touch_events.read().count() > 0;
    let has_cursor = cursor_events.read().count() > 0;

    if has_touch {
        touch_active.0 = true;
    } else if has_cursor && !has_touch {
        touch_active.0 = false;
    }
}

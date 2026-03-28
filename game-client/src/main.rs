#![allow(dead_code)]

use bevy::{asset::AssetPlugin, prelude::*};
use game_core::systems::DeltaTime;

// ─── dev_warn! 条件编译宏 ───

#[cfg(feature = "dev_log")]
macro_rules! dev_warn {
    ($($arg:tt)*) => { web_sys::console::warn_1(&format!($($arg)*).into()) };
}
#[cfg(not(feature = "dev_log"))]
macro_rules! dev_warn {
    ($($arg:tt)*) => {};
}

pub(crate) use dev_warn;

mod arena;
mod audio;
mod egui_setup;
mod imprint;
mod input;
pub mod js_bridge;
mod menu;
mod menu_panels;
mod render;
mod simulation;
mod storage;
mod sync;
mod ui;

// ─── 客户端 GameState 状态机 ───

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        canvas: Some("#game-canvas".to_string()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    // WASM 中资源路径不需要审批
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .init_state::<GameState>()
        .add_plugins(egui_setup::EguiSetupPlugin)
        .add_plugins(menu::MenuPlugin)
        .add_plugins(simulation::SimulationPlugin)
        .add_plugins(input::InputPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(sync::SyncPlugin)
        .add_plugins(imprint::ImprintPlugin)
        .add_plugins(arena::ArenaPlugin)
        .add_plugins(menu_panels::MenuPanelsPlugin)
        .add_plugins(audio::AudioPlugin)
        .insert_resource(ClearColor(Color::srgb(0.012, 0.020, 0.031)))
        .insert_resource(DeltaTime(game_core::data::constants::TICK_DT))
        .run();
}

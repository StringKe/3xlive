//! UI 模块：HUD、背包、Disc 编辑器

pub mod components;
pub mod drag;
pub mod game;
pub mod hud;
pub mod inventory;
pub mod virtual_joystick;

use crate::GameState;
use bevy::prelude::*;
use game_core::data::disc::{Disc, DiscQuality};
use game_core::data::program::{Program, ProgramParams, ProgramType};
use std::collections::VecDeque;

// ─── InputMode 资源 ───

/// 输入模式：游戏模式 / UI 模式
#[derive(Resource, Debug, Clone, Default)]
pub enum InputMode {
    #[default]
    GameMode,
    UiMode {
        locked_aim_x: f32,
        locked_aim_y: f32,
    },
}

// ─── 客户端本地背包状态 ───

/// 客户端物品（Disc 或 Program）
#[derive(Clone, Debug)]
pub enum ClientItem {
    Disc(ClientDisc),
    Program(ClientProgram),
}

/// 客户端 Disc 数据
#[derive(Clone, Debug)]
pub struct ClientDisc {
    pub quality: DiscQuality,
    pub slots: Vec<Option<ClientProgram>>,
    pub base_fire_rate: f32,
    pub base_spread: f32,
}

/// 客户端 Program 数据（保留完整 params 用于同步回 ECS）
#[derive(Clone, Debug)]
pub struct ClientProgram {
    pub program_id: u32,
    pub program_type: ProgramType,
    pub params: ProgramParams,
}

impl From<ClientProgram> for Program {
    fn from(cp: ClientProgram) -> Self {
        Program {
            program_id: cp.program_id,
            program_type: cp.program_type,
            params: cp.params,
        }
    }
}

impl From<ClientDisc> for Disc {
    fn from(cd: ClientDisc) -> Self {
        Disc {
            quality: cd.quality,
            slots: cd
                .slots
                .into_iter()
                .map(|opt| opt.map(|p| p.into()))
                .collect(),
            base_fire_rate: cd.base_fire_rate,
            base_spread: cd.base_spread,
        }
    }
}

/// 客户端本地背包 Resource
#[derive(Resource, Debug, Clone)]
pub struct ClientInventory {
    pub slots: Vec<Option<ClientItem>>,
    pub max_slots: usize,
    pub equipped_disc: Option<ClientDisc>,
}

impl Default for ClientInventory {
    fn default() -> Self {
        Self {
            slots: vec![None; 24],
            max_slots: 24,
            equipped_disc: None,
        }
    }
}

impl ClientInventory {
    /// 背包初始容量
    pub const INITIAL_SLOTS: usize = game_core::data::constants::INVENTORY_INITIAL_SLOTS;
    /// 背包最大容量
    pub const MAX_SLOTS: usize = game_core::data::constants::INVENTORY_MAX_SLOTS;

    /// 根据等级计算背包容量：初始 24 格，每升 1 级解锁 1 格，上限 128
    pub fn slots_for_level(level: u32) -> usize {
        (Self::INITIAL_SLOTS + level.saturating_sub(1) as usize).min(Self::MAX_SLOTS)
    }

    /// 扩容背包到指定容量
    pub fn resize(&mut self, new_max: usize) {
        if new_max > self.max_slots {
            self.slots.resize(new_max, None);
            self.max_slots = new_max;
        }
    }
}

// ─── 聊天/日志系统 ───

/// 聊天消息条目
#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub sender: String,
    pub text: String,
    pub kind: ChatMessageKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatMessageKind {
    System,
    LocalPlayer,
    Ally,
    Enemy,
}

/// 聊天状态 Resource
#[derive(Resource, Debug, Clone)]
pub struct ChatState {
    pub messages: VecDeque<ChatMessage>,
    pub input_buf: String,
    pub max_messages: usize,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
            input_buf: String::new(),
            max_messages: 50,
        }
    }
}

impl ChatState {
    pub fn push(&mut self, msg: ChatMessage) {
        if self.messages.len() >= self.max_messages {
            self.messages.pop_front();
        }
        self.messages.push_back(msg);
    }
}

// ─── 玩家统计 ───

/// 本地玩家战绩统计
#[derive(Resource, Debug, Clone, Default)]
pub struct PlayerStats {
    pub kills: u32,
}

// ─── 本地玩家信息 ───

/// 本地玩家元信息（display_name 等）
#[derive(Resource, Debug, Clone)]
pub struct LocalPlayerInfo {
    pub display_name: String,
}

impl Default for LocalPlayerInfo {
    fn default() -> Self {
        Self {
            display_name: "Player".to_string(),
        }
    }
}

// ─── 背包翻页 ───

/// 背包翻页状态
#[derive(Resource, Debug, Clone, Default)]
pub struct BackpackPager {
    pub page: usize,
}

// ─── 插件 ───

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputMode>()
            .init_resource::<ClientInventory>()
            .init_resource::<ChatState>()
            .init_resource::<PlayerStats>()
            .init_resource::<LocalPlayerInfo>()
            .init_resource::<BackpackPager>()
            .add_plugins(hud::HudPlugin)
            .add_plugins(drag::DragPlugin)
            .add_plugins(virtual_joystick::VirtualJoystickPlugin)
            .add_systems(
                Update,
                toggle_ui_system.run_if(in_state(GameState::Playing)),
            );
    }
}

/// 按 Tab / I 切换 InputMode
fn toggle_ui_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<InputMode>,
    local_player: Option<Res<game_core::resources::LocalPlayerEntity>>,
    aim_query: Query<&game_core::components::AimDirection>,
) {
    if keys.just_pressed(KeyCode::Tab) || keys.just_pressed(KeyCode::KeyI) {
        *mode = match *mode {
            InputMode::GameMode => {
                let (aim_x, aim_y) = local_player
                    .and_then(|lp| aim_query.get(lp.entity).ok())
                    .map(|aim| (aim.x, aim.y))
                    .unwrap_or((1.0, 0.0));
                InputMode::UiMode {
                    locked_aim_x: aim_x,
                    locked_aim_y: aim_y,
                }
            }
            InputMode::UiMode { .. } => InputMode::GameMode,
        };
    }
}

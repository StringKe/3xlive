//! 世界印记系统：死亡标记、路标放置、印记渲染
//!
//! 印记通过 IndexedDB 持久化，sync 时上传服务端。
//! 其他玩家的印记通过 pullState 下载后缓存到本地。

use bevy::prelude::*;
use bevy_egui::EguiContexts;

use game_core::components::Position;
use game_core::events::GameEvent;
use game_core::resources::LocalPlayerEntity;
use game_core::systems::imprint::{ImprintType, WorldImprint};

use crate::storage;
use crate::GameState;

/// 渲染用的印记标记组件
#[derive(Component)]
struct ImprintMarker {
    imprint_type: ImprintType,
}

/// 本局放置的路标计数器
#[derive(Resource, Default)]
struct WaypointCounter(u32);

/// 最大路标数
const MAX_WAYPOINTS_PER_RUN: u32 = 3;

pub struct ImprintPlugin;

impl Plugin for ImprintPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaypointCounter>()
            .add_systems(OnEnter(GameState::Playing), reset_waypoint_counter)
            // 纯 ECS 系统
            .add_systems(
                Update,
                (death_imprint_system, imprint_pulse_animation)
                    .run_if(in_state(GameState::Playing)),
            )
            // egui 系统：检查 ctx.wants_keyboard_input()，需在 EguiPrimaryContextPass 中运行
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                waypoint_input_system.run_if(in_state(GameState::Playing)),
            );
    }
}

fn reset_waypoint_counter(mut counter: ResMut<WaypointCounter>) {
    counter.0 = 0;
}

/// 7.1.2 监听 EntityKilled（本地玩家死亡），自动创建 DeathMark
fn death_imprint_system(
    mut events: MessageReader<GameEvent>,
    local_player: Option<Res<LocalPlayerEntity>>,
    positions: Query<&Position>,
) {
    let Some(local_player) = local_player else {
        return;
    };

    for event in events.read() {
        if let GameEvent::EntityKilled { entity, .. } = event {
            if *entity == local_player.entity {
                if let Ok(pos) = positions.get(*entity) {
                    let imprint = WorldImprint {
                        imprint_type: ImprintType::DeathMark,
                        x: pos.x,
                        y: pos.y,
                        creator_id: 1,
                        data: String::new(),
                        timestamp: js_sys::Date::now() as u64,
                    };
                    let config = bincode::config::standard();
                    if let Ok(bytes) = bincode::encode_to_vec(&imprint, config) {
                        let key = format!("imprint_{}", imprint.timestamp);
                        storage::write_pending_upload(&key, &bytes);
                    }
                }
            }
        }
    }
}

/// 7.1.3 V 键放置路标（每局限 3 个）
#[allow(clippy::too_many_arguments)]
fn waypoint_input_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut counter: ResMut<WaypointCounter>,
    local_player: Option<Res<LocalPlayerEntity>>,
    positions: Query<&Position>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut contexts: EguiContexts,
) {
    if contexts
        .ctx_mut()
        .is_ok_and(|ctx| ctx.wants_keyboard_input())
    {
        return;
    }

    if !keys.just_pressed(KeyCode::KeyV) {
        return;
    }

    if counter.0 >= MAX_WAYPOINTS_PER_RUN {
        return;
    }

    let Some(local_player) = local_player else {
        return;
    };

    let Ok(pos) = positions.get(local_player.entity) else {
        return;
    };

    counter.0 += 1;

    // spawn 路标视觉实体
    let mesh = meshes.add(Rectangle::new(8.0, 32.0));
    // 青色光柱（HDR 高亮）
    let material = materials.add(ColorMaterial::from(Color::srgb(0.0, 8.0, 10.0)));

    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_translation(Vec3::new(pos.x, pos.y, 5.0)),
        ImprintMarker {
            imprint_type: ImprintType::Waypoint,
        },
    ));

    // 持久化
    let imprint = WorldImprint {
        imprint_type: ImprintType::Waypoint,
        x: pos.x,
        y: pos.y,
        creator_id: 1,
        data: String::new(),
        timestamp: js_sys::Date::now() as u64,
    };
    let config = bincode::config::standard();
    if let Ok(bytes) = bincode::encode_to_vec(&imprint, config) {
        let key = format!("imprint_{}", imprint.timestamp);
        storage::write_pending_upload(&key, &bytes);
    }
}

/// 7.1.4 印记脉冲动画（呼吸缩放）
fn imprint_pulse_animation(time: Res<Time>, mut query: Query<(&mut Transform, &ImprintMarker)>) {
    let t = time.elapsed_secs();
    for (mut transform, marker) in query.iter_mut() {
        let base_scale = match marker.imprint_type {
            ImprintType::DeathMark => 1.0,
            ImprintType::Waypoint => 1.0,
            ImprintType::LootCache => 0.8,
            ImprintType::DangerZone => 1.2,
        };
        let pulse = 1.0 + 0.15 * (t * 2.0).sin();
        let s = base_scale * pulse;
        transform.scale = Vec3::new(s, s, 1.0);
    }
}

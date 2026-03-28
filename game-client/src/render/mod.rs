use crate::GameState;
use bevy::prelude::*;
use bevy::sprite_render::Material2dPlugin;

pub mod camera;
pub mod effects;
pub mod entity_shapes;
pub mod grid_material;
pub mod zone_effects;

use camera::{camera_follow_system, setup_camera, GameCamera};
use effects::{
    damage_number_system, death_animation_spawner, death_fragment_system, hit_confirm_handler,
    hit_flash_system, loot_animation_system, setup_effect_assets,
};
use entity_shapes::{
    attach_loot_visual, attach_monster_visual, attach_player_visual, attach_projectile_visual,
    init_previous_position, setup_entity_assets, sync_position_to_transform,
};
use grid_material::{GridMaterial, GridUniforms};

/// 网格地面实体标记
#[derive(Component)]
struct GridQuad;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<GridMaterial>::default())
            .add_systems(
                Startup,
                (
                    setup_camera,
                    setup_grid,
                    setup_entity_assets,
                    setup_effect_assets,
                ),
            )
            // 背景渲染（所有状态）
            .add_systems(
                Update,
                (
                    camera_follow_system,
                    update_grid_uniforms,
                    grid_follow_camera,
                ),
            )
            // 游戏特效（仅 Playing 状态）
            .add_systems(
                Update,
                (
                    attach_player_visual,
                    attach_monster_visual,
                    attach_projectile_visual,
                    attach_loot_visual,
                    init_previous_position,
                    sync_position_to_transform,
                    // hit_confirm_handler 必须在 game_event_bridge 之后执行，
                    // 否则读不到当帧转发的 GameEvent
                    hit_confirm_handler.after(crate::simulation::game_event_bridge),
                    damage_number_system,
                    hit_flash_system,
                    death_animation_spawner.after(attach_monster_visual),
                    death_fragment_system,
                    loot_animation_system,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// 创建全屏四边形，绑定 GridMaterial
fn setup_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GridMaterial>>,
) {
    let mesh = meshes.add(Rectangle::new(1.0, 1.0));
    let material = materials.add(GridMaterial::default());

    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_translation(Vec3::new(0.0, 0.0, -100.0))
            .with_scale(Vec3::new(4096.0, 4096.0, 1.0)),
        GridQuad,
    ));
}

/// 每帧更新 GridMaterial 的 uniform（摄像机位置、视口大小、时间）
fn update_grid_uniforms(
    time: Res<Time>,
    camera_query: Query<(&Transform, &Projection), With<GameCamera>>,
    grid_query: Query<(&MeshMaterial2d<GridMaterial>, &Transform), With<GridQuad>>,
    mut materials: ResMut<Assets<GridMaterial>>,
    _windows: Query<&bevy::window::Window>,
) {
    let Ok((cam_transform, _projection)) = camera_query.single() else {
        return;
    };
    let Ok((mat_handle, grid_transform)) = grid_query.single() else {
        return;
    };

    let Some(material) = materials.get_mut(&mat_handle.0) else {
        return;
    };

    let cam_pos = cam_transform.translation.truncate();
    // view_size = quad 世界尺寸（mesh 1x1 * scale），UV 完整映射到这个范围
    let view_size = grid_transform.scale.truncate();

    material.uniforms = GridUniforms {
        camera_pos: cam_pos,
        view_size,
        time: time.elapsed_secs(),
        tile_size: game_core::data::constants::TILE_SIZE,
        _padding: Vec2::ZERO,
    };
}

/// 让 grid quad 跟随摄像机 XY，确保始终覆盖视口
fn grid_follow_camera(
    camera_query: Query<&Transform, With<GameCamera>>,
    mut grid_query: Query<&mut Transform, (With<GridQuad>, Without<GameCamera>)>,
) {
    let Ok(cam_transform) = camera_query.single() else {
        return;
    };
    let Ok(mut grid_transform) = grid_query.single_mut() else {
        return;
    };
    grid_transform.translation.x = cam_transform.translation.x;
    grid_transform.translation.y = cam_transform.translation.y;
}

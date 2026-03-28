use bevy::prelude::*;

use game_core::components::{LootDrop, LootItem, MonsterComp, Position};
use game_core::data::disc::DiscQuality;
use game_core::data::program::ProgramType;

use super::effects::{BossClientMarker, EliteClientMarker, LootAssets, LootMarker, LootVisualType};
use crate::sync::LocalPlayerMarker;

// ─── 组件 ───

/// 渲染形状标记，用于区分实体类型的视觉表现
#[derive(Component)]
pub struct RenderShape {
    pub shape: ShapeType,
}

/// 上一 tick 的位置，用于 FixedUpdate → Update 渲染插值
#[derive(Component, Clone, Copy)]
pub struct PreviousPosition {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeType {
    Hexagon,
    Triangle,
    Square,
    Diamond,
}

/// 标记怪物实体（渲染用）
#[derive(Component)]
pub struct MonsterMarker;

// ─── 资源：预加载的 mesh 和 material handles ───

#[derive(Resource)]
pub struct EntityMeshes {
    pub hexagon: Handle<Mesh>,
    pub triangle: Handle<Mesh>,
    pub square: Handle<Mesh>,
    pub diamond: Handle<Mesh>,
    pub projectile: Handle<Mesh>,
}

#[derive(Resource)]
pub struct EntityMaterials {
    /// 本地玩家：亮 Cyan（HDR 高亮度产生 bloom）
    pub local_player: Handle<ColorMaterial>,
    /// 怪物：绿色三角
    pub monster_green: Handle<ColorMaterial>,
    /// 怪物：紫色方块
    pub monster_purple: Handle<ColorMaterial>,
    /// 精英怪物：黄色 HDR
    pub elite_yellow: Handle<ColorMaterial>,
    /// Boss：红白 HDR
    pub boss_red: Handle<ColorMaterial>,
    /// 弹射体：亮橙色
    pub projectile_orange: Handle<ColorMaterial>,
}

/// Startup 系统：预创建所有 mesh 和 material
pub fn setup_entity_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // 六边形 mesh（玩家）
    let hexagon = meshes.add(RegularPolygon::new(16.0, 6));
    // 三角形 mesh（怪物类型 A）
    let triangle = meshes.add(RegularPolygon::new(14.0, 3));
    // 方块 mesh（怪物类型 B）
    let square = meshes.add(Rectangle::new(22.0, 22.0));
    // 菱形 mesh（怪物类型 C）= 旋转 45 度的方块，用同一个 mesh
    let diamond = meshes.add(Rectangle::new(18.0, 18.0));
    // 弹射体 mesh：小圆形
    let projectile = meshes.add(Circle::new(4.0));

    commands.insert_resource(EntityMeshes {
        hexagon,
        triangle,
        square,
        diamond,
        projectile,
    });

    // 材质：SDR 颜色
    let local_player = materials.add(ColorMaterial::from_color(Color::srgb(0.0, 0.9, 1.0)));
    let monster_green = materials.add(ColorMaterial::from_color(Color::srgb(0.15, 0.8, 0.3)));
    let monster_purple = materials.add(ColorMaterial::from_color(Color::srgb(0.6, 0.2, 0.8)));
    let elite_yellow = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.85, 0.2)));
    let boss_red = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.4, 0.4)));
    let projectile_orange = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.7, 0.2)));

    commands.insert_resource(EntityMaterials {
        local_player,
        monster_green,
        monster_purple,
        elite_yellow,
        boss_red,
        projectile_orange,
    });
}

/// 为本地玩家添加渲染组件
pub fn attach_player_visual(
    mut commands: Commands,
    meshes: Res<EntityMeshes>,
    materials: Res<EntityMaterials>,
    new_players: Query<(Entity, &Position), Added<LocalPlayerMarker>>,
) {
    for (entity, pos) in &new_players {
        let Ok(mut cmd) = commands.get_entity(entity) else {
            continue;
        };
        cmd.try_insert((
            Mesh2d(meshes.hexagon.clone()),
            MeshMaterial2d(materials.local_player.clone()),
            Transform::from_xyz(pos.x, pos.y, 1.0),
            RenderShape {
                shape: ShapeType::Hexagon,
            },
        ));
    }
}

/// 为新 spawn 的怪物添加渲染组件
#[allow(clippy::type_complexity)]
pub fn attach_monster_visual(
    mut commands: Commands,
    meshes: Res<EntityMeshes>,
    materials: Res<EntityMaterials>,
    new_monsters: Query<
        (
            Entity,
            &Position,
            &MonsterComp,
            Option<&EliteClientMarker>,
            Option<&BossClientMarker>,
        ),
        Added<MonsterComp>,
    >,
) {
    for (entity, pos, monster, is_elite, is_boss) in &new_monsters {
        let Ok(mut cmd) = commands.get_entity(entity) else {
            continue;
        };

        if is_boss.is_some() {
            cmd.try_insert((
                Mesh2d(meshes.hexagon.clone()),
                MeshMaterial2d(materials.boss_red.clone()),
                Transform::from_xyz(pos.x, pos.y, 0.5).with_scale(Vec3::splat(3.0)),
                RenderShape {
                    shape: ShapeType::Hexagon,
                },
                MonsterMarker,
            ));
        } else if is_elite.is_some() {
            cmd.try_insert((
                Mesh2d(meshes.diamond.clone()),
                MeshMaterial2d(materials.elite_yellow.clone()),
                Transform::from_xyz(pos.x, pos.y, 0.5).with_scale(Vec3::splat(1.3)),
                RenderShape {
                    shape: ShapeType::Diamond,
                },
                MonsterMarker,
            ));
        } else {
            // 普通怪物：根据 monster_type 奇偶选形状
            let is_type_a = monster.monster_type % 2 == 0;
            if is_type_a {
                cmd.try_insert((
                    Mesh2d(meshes.triangle.clone()),
                    MeshMaterial2d(materials.monster_green.clone()),
                    Transform::from_xyz(pos.x, pos.y, 0.5),
                    RenderShape {
                        shape: ShapeType::Triangle,
                    },
                    MonsterMarker,
                ));
            } else {
                cmd.try_insert((
                    Mesh2d(meshes.square.clone()),
                    MeshMaterial2d(materials.monster_purple.clone()),
                    Transform::from_xyz(pos.x, pos.y, 0.5),
                    RenderShape {
                        shape: ShapeType::Square,
                    },
                    MonsterMarker,
                ));
            }
        }
    }
}

/// 为新 spawn 的弹射体添加渲染组件
pub fn attach_projectile_visual(
    mut commands: Commands,
    meshes: Res<EntityMeshes>,
    materials: Res<EntityMaterials>,
    new_projectiles: Query<(Entity, &Position), Added<game_core::components::ProjectileComp>>,
) {
    for (entity, pos) in &new_projectiles {
        let Ok(mut cmd) = commands.get_entity(entity) else {
            continue;
        };
        cmd.try_insert((
            Mesh2d(meshes.projectile.clone()),
            MeshMaterial2d(materials.projectile_orange.clone()),
            Transform::from_xyz(pos.x, pos.y, 0.9),
            RenderShape {
                shape: ShapeType::Diamond,
            },
        ));
    }
}

/// 为新 spawn 的掉落物添加渲染组件
pub fn attach_loot_visual(
    mut commands: Commands,
    loot_assets: Option<Res<LootAssets>>,
    new_loot: Query<(Entity, &Position, &LootDrop), Added<LootDrop>>,
) {
    let Some(assets) = loot_assets else { return };

    for (entity, pos, loot) in &new_loot {
        let Ok(mut cmd) = commands.get_entity(entity) else {
            continue;
        };

        let (mesh, mat, visual_type) = match &loot.item {
            LootItem::DiscDrop(disc) => {
                let mat = match disc.quality {
                    DiscQuality::Common => assets.disc_mat_common.clone(),
                    DiscQuality::Uncommon => assets.disc_mat_uncommon.clone(),
                    DiscQuality::Rare => assets.disc_mat_rare.clone(),
                    DiscQuality::Epic => assets.disc_mat_epic.clone(),
                    DiscQuality::Legendary => assets.disc_mat_legendary.clone(),
                };
                (assets.disc_mesh.clone(), mat, LootVisualType::Disc)
            }
            LootItem::ProgramDrop(prog) => {
                let (mesh, mat) = match prog.program_type {
                    ProgramType::Projectile => (
                        assets.prog_projectile_mesh.clone(),
                        assets.prog_projectile_mat.clone(),
                    ),
                    ProgramType::TrajectoryMod => (
                        assets.prog_trajectory_mesh.clone(),
                        assets.prog_trajectory_mat.clone(),
                    ),
                    ProgramType::QuantityMod => (
                        assets.prog_quantity_mesh.clone(),
                        assets.prog_quantity_mat.clone(),
                    ),
                    ProgramType::Trigger => (
                        assets.prog_trigger_mesh.clone(),
                        assets.prog_trigger_mat.clone(),
                    ),
                    ProgramType::StatusEffect => (
                        assets.prog_status_mesh.clone(),
                        assets.prog_status_mat.clone(),
                    ),
                    ProgramType::FormationMod => (
                        assets.prog_formation_mesh.clone(),
                        assets.prog_formation_mat.clone(),
                    ),
                    ProgramType::TimingMod => (
                        assets.prog_timing_mesh.clone(),
                        assets.prog_timing_mat.clone(),
                    ),
                    ProgramType::DamageMod => (
                        assets.prog_damage_mod_mesh.clone(),
                        assets.prog_damage_mod_mat.clone(),
                    ),
                    ProgramType::ProjectileMod => (
                        assets.prog_projectile_mod_mesh.clone(),
                        assets.prog_projectile_mod_mat.clone(),
                    ),
                    ProgramType::FieldMod => (
                        assets.prog_field_mesh.clone(),
                        assets.prog_field_mat.clone(),
                    ),
                };
                (mesh, mat, LootVisualType::Program)
            }
            LootItem::BuffDrop(_) => (
                assets.buff_mesh.clone(),
                assets.buff_mat.clone(),
                LootVisualType::Buff,
            ),
            LootItem::XpOrb(_) => (
                assets.xp_mesh.clone(),
                assets.xp_mat.clone(),
                LootVisualType::XpOrb,
            ),
        };

        // 用 entity index 生成伪随机相位偏移
        let phase_offset = (entity.index_u32() as f32 * 2.654) % std::f32::consts::TAU;
        cmd.try_insert((
            Mesh2d(mesh),
            MeshMaterial2d(mat),
            Transform::from_xyz(pos.x, pos.y, 0.8),
            RenderShape {
                shape: ShapeType::Diamond,
            },
            LootMarker {
                loot_type: visual_type,
                phase_offset,
            },
        ));
    }
}

/// 为新增的 RenderShape 实体自动添加 PreviousPosition
pub fn init_previous_position(
    mut commands: Commands,
    query: Query<(Entity, &Position), Added<RenderShape>>,
) {
    for (entity, pos) in &query {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.try_insert(PreviousPosition { x: pos.x, y: pos.y });
        }
    }
}

/// 每帧插值 Position -> Transform（消除 20Hz FixedUpdate 抖动）
#[allow(clippy::type_complexity)]
pub fn sync_position_to_transform(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(&Position, Option<&PreviousPosition>, &mut Transform), With<RenderShape>>,
) {
    let alpha = fixed_time.overstep_fraction();
    for (pos, prev, mut transform) in &mut query {
        if let Some(prev) = prev {
            transform.translation.x = prev.x + (pos.x - prev.x) * alpha;
            transform.translation.y = prev.y + (pos.y - prev.y) * alpha;
        } else {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
        // z 保持不变
    }
}

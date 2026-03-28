//! 视觉特效系统：伤害数字、命中闪光、死亡碎片、掉落物渲染

use bevy::prelude::*;

use game_core::components::Position;
use game_core::events::GameEvent;

use super::entity_shapes::MonsterMarker;

// ─── 组件 ───

/// 精英怪物标记：脉冲缩放效果
#[derive(Component)]
pub struct EliteClientMarker;

/// Boss 标记：呼吸发光效果
#[derive(Component)]
pub struct BossClientMarker;

/// 伤害数字：世界坐标中上浮 + 淡出，1 秒后 despawn
#[derive(Component)]
pub struct DamageNumber {
    pub timer: Timer,
    pub start_y: f32,
    /// 上浮总距离（像素）
    pub float_distance: f32,
}

/// 命中闪光：高亮度白色方块，0.1 秒后 despawn
#[derive(Component)]
pub struct HitFlash {
    pub timer: Timer,
}

/// 死亡碎片：向外散开 + 淡出，0.5 秒后 despawn
#[derive(Component)]
pub struct DeathFragment {
    pub timer: Timer,
    pub vel_x: f32,
    pub vel_y: f32,
}

/// 掉落物标记，驱动旋转 + 呼吸发光
#[derive(Component)]
pub struct LootMarker {
    pub loot_type: LootVisualType,
    /// 相位偏移，使不同掉落物动画错开
    pub phase_offset: f32,
}

/// 掉落物视觉类型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LootVisualType {
    /// Disc 掉落：菱形
    Disc,
    /// Program 掉落：小三角
    Program,
    /// Buff 掉落：圆形
    Buff,
    /// 经验球：点
    XpOrb,
}

// ─── 常量 ───

/// 伤害数字 HDR 橙色（用函数包装，因为 Color::srgb 非 const fn）
fn damage_number_color() -> Color {
    Color::srgb(1.0, 0.5, 0.15)
}

// ─── 资源 ───

/// 掉落物专用 mesh 和 material
#[derive(Resource)]
pub struct LootAssets {
    // Disc（按品质区分颜色）
    pub disc_mesh: Handle<Mesh>,
    pub disc_mat_common: Handle<ColorMaterial>,
    pub disc_mat_uncommon: Handle<ColorMaterial>,
    pub disc_mat_rare: Handle<ColorMaterial>,
    pub disc_mat_epic: Handle<ColorMaterial>,
    pub disc_mat_legendary: Handle<ColorMaterial>,
    // Program（每种类型独立形状 + 颜色）
    pub prog_projectile_mesh: Handle<Mesh>,
    pub prog_projectile_mat: Handle<ColorMaterial>,
    pub prog_trajectory_mesh: Handle<Mesh>,
    pub prog_trajectory_mat: Handle<ColorMaterial>,
    pub prog_quantity_mesh: Handle<Mesh>,
    pub prog_quantity_mat: Handle<ColorMaterial>,
    pub prog_trigger_mesh: Handle<Mesh>,
    pub prog_trigger_mat: Handle<ColorMaterial>,
    pub prog_status_mesh: Handle<Mesh>,
    pub prog_status_mat: Handle<ColorMaterial>,
    pub prog_formation_mesh: Handle<Mesh>,
    pub prog_formation_mat: Handle<ColorMaterial>,
    pub prog_timing_mesh: Handle<Mesh>,
    pub prog_timing_mat: Handle<ColorMaterial>,
    pub prog_damage_mod_mesh: Handle<Mesh>,
    pub prog_damage_mod_mat: Handle<ColorMaterial>,
    pub prog_projectile_mod_mesh: Handle<Mesh>,
    pub prog_projectile_mod_mat: Handle<ColorMaterial>,
    pub prog_field_mesh: Handle<Mesh>,
    pub prog_field_mat: Handle<ColorMaterial>,
    // Buff & XP
    pub buff_mesh: Handle<Mesh>,
    pub buff_mat: Handle<ColorMaterial>,
    pub xp_mesh: Handle<Mesh>,
    pub xp_mat: Handle<ColorMaterial>,
}

/// 特效专用 mesh 和 material
#[derive(Resource)]
pub struct EffectAssets {
    /// 命中闪光用的小方块 mesh
    pub flash_mesh: Handle<Mesh>,
    /// HDR 白色高亮 material
    pub flash_mat: Handle<ColorMaterial>,
    /// 死亡碎片用的小三角 mesh
    pub fragment_mesh: Handle<Mesh>,
    /// 死亡碎片 material（亮绿）
    pub fragment_mat_green: Handle<ColorMaterial>,
    /// 死亡碎片 material（亮紫）
    pub fragment_mat_purple: Handle<ColorMaterial>,
}

// ─── 启动 ───

/// 预创建掉落物和特效的 mesh/material
pub fn setup_effect_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Disc 掉落 mesh（菱形）
    let disc_mesh = meshes.add(Rectangle::new(10.0, 10.0));
    // Disc 品质颜色
    let disc_mat_common = materials.add(ColorMaterial::from_color(Color::srgb(0.7, 0.7, 0.7)));
    let disc_mat_uncommon = materials.add(ColorMaterial::from_color(Color::srgb(0.2, 0.8, 0.2)));
    let disc_mat_rare = materials.add(ColorMaterial::from_color(Color::srgb(0.3, 0.5, 1.0)));
    let disc_mat_epic = materials.add(ColorMaterial::from_color(Color::srgb(0.7, 0.25, 0.85)));
    let disc_mat_legendary = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.8, 0.1)));

    // Program 掉落：每种类型独立形状 + 颜色
    let prog_projectile_mesh = meshes.add(Circle::new(7.0));
    let prog_projectile_mat = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.7, 0.2)));
    let prog_trajectory_mesh = meshes.add(RegularPolygon::new(8.0, 3));
    let prog_trajectory_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.3, 0.5, 1.0)));
    let prog_quantity_mesh = meshes.add(Rectangle::new(8.0, 8.0));
    let prog_quantity_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.3, 0.5, 1.0)));
    let prog_trigger_mesh = meshes.add(RegularPolygon::new(8.0, 5));
    let prog_trigger_mat = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.3, 0.3)));
    let prog_status_mesh = meshes.add(RegularPolygon::new(7.0, 6));
    let prog_status_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.3, 1.0, 0.3)));

    // FormationMod: 十字形（4边矩形）
    let prog_formation_mesh = meshes.add(RegularPolygon::new(8.0, 4));
    let prog_formation_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.0, 0.8, 0.8)));
    // TimingMod: 七边形
    let prog_timing_mesh = meshes.add(RegularPolygon::new(7.0, 7));
    let prog_timing_mat = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.6, 0.0)));
    // DamageMod: 小菱形
    let prog_damage_mod_mesh = meshes.add(Rectangle::new(6.0, 10.0));
    let prog_damage_mod_mat = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 0.2, 0.2)));
    // ProjectileMod: 八边形
    let prog_projectile_mod_mesh = meshes.add(RegularPolygon::new(7.0, 8));
    let prog_projectile_mod_mat =
        materials.add(ColorMaterial::from_color(Color::srgb(0.2, 1.0, 0.2)));
    // FieldMod: 大圆
    let prog_field_mesh = meshes.add(Circle::new(9.0));
    let prog_field_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.8, 0.0, 0.8)));

    // Buff & XP
    let buff_mesh = meshes.add(Circle::new(6.0));
    let buff_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.7, 0.25, 0.85)));
    let xp_mesh = meshes.add(Circle::new(4.0));
    let xp_mat = materials.add(ColorMaterial::from_color(Color::srgb(0.2, 1.0, 0.2)));

    commands.insert_resource(LootAssets {
        disc_mesh,
        disc_mat_common,
        disc_mat_uncommon,
        disc_mat_rare,
        disc_mat_epic,
        disc_mat_legendary,
        prog_projectile_mesh,
        prog_projectile_mat,
        prog_trajectory_mesh,
        prog_trajectory_mat,
        prog_quantity_mesh,
        prog_quantity_mat,
        prog_trigger_mesh,
        prog_trigger_mat,
        prog_status_mesh,
        prog_status_mat,
        prog_formation_mesh,
        prog_formation_mat,
        prog_timing_mesh,
        prog_timing_mat,
        prog_damage_mod_mesh,
        prog_damage_mod_mat,
        prog_projectile_mod_mesh,
        prog_projectile_mod_mat,
        prog_field_mesh,
        prog_field_mat,
        buff_mesh,
        buff_mat,
        xp_mesh,
        xp_mat,
    });

    // 特效 mesh/material
    let flash_mesh = meshes.add(Rectangle::new(8.0, 8.0));
    let flash_mat = materials.add(ColorMaterial::from_color(Color::srgb(1.0, 1.0, 1.0)));
    let fragment_mesh = meshes.add(RegularPolygon::new(5.0, 3));
    let fragment_mat_green = materials.add(ColorMaterial::from_color(Color::srgb(0.15, 0.8, 0.3)));
    let fragment_mat_purple = materials.add(ColorMaterial::from_color(Color::srgb(0.6, 0.2, 0.8)));

    commands.insert_resource(EffectAssets {
        flash_mesh,
        flash_mat,
        fragment_mesh,
        fragment_mat_green,
        fragment_mat_purple,
    });
}

// ─── 系统 ───

/// 收到 EntityDamaged 事件后在 target 位置播放命中闪光 + 生成伤害数字
pub fn hit_confirm_handler(
    mut commands: Commands,
    mut reader: MessageReader<GameEvent>,
    positions: Query<&Position>,
    effect_assets: Option<Res<EffectAssets>>,
    game_font: Option<Res<crate::menu::GameFont>>,
) {
    let Some(assets) = effect_assets else { return };

    for ev in reader.read() {
        if let GameEvent::EntityDamaged { entity, damage, .. } = ev {
            let Ok(pos) = positions.get(*entity) else {
                continue;
            };

            // 命中闪光：高亮度白色小方块，z=8 确保高于所有怪物/玩家实体
            commands.spawn((
                Mesh2d(assets.flash_mesh.clone()),
                MeshMaterial2d(assets.flash_mat.clone()),
                Transform::from_xyz(pos.x, pos.y, 8.0),
                HitFlash {
                    timer: Timer::from_seconds(0.1, TimerMode::Once),
                },
            ));

            // 伤害数字：使用 Text2d 在世界坐标中显示
            let text = format!("{:.0}", damage);
            let font = TextFont {
                font: game_font.as_ref().map(|f| f.0.clone()).unwrap_or_default(),
                font_size: 18.0,
                ..default()
            };
            commands.spawn((
                Text2d::new(text),
                font,
                TextColor(damage_number_color()),
                Transform::from_xyz(pos.x, pos.y + 10.0, 10.0),
                DamageNumber {
                    timer: Timer::from_seconds(1.0, TimerMode::Once),
                    start_y: pos.y + 10.0,
                    float_distance: 40.0,
                },
            ));
        }
    }
}

/// 伤害数字：上浮 + 淡出动画，到期 despawn
pub fn damage_number_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DamageNumber, &mut Transform, &mut TextColor)>,
) {
    for (entity, mut dn, mut transform, mut color) in &mut query {
        dn.timer.tick(time.delta());
        let t = dn.timer.fraction();

        // 上浮
        transform.translation.y = dn.start_y + t * dn.float_distance;

        // 淡出：alpha 从 1.0 线性降到 0.0
        let alpha = 1.0 - t;
        color.0 = damage_number_color().with_alpha(alpha);

        if dn.timer.is_finished() {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
        }
    }
}

/// 命中闪光：缩放从 1.0 到 0.0 + 到期 despawn
pub fn hit_flash_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut HitFlash, &mut Transform)>,
) {
    for (entity, mut flash, mut transform) in &mut query {
        flash.timer.tick(time.delta());
        let t = flash.timer.fraction();
        let scale = 1.0 - t;
        transform.scale = Vec3::splat(scale);
        if flash.timer.is_finished() {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
        }
    }
}

/// 怪物死亡时 spawn 死亡碎片
/// 监听 EntityKilled 事件，在怪物位置 spawn 碎片
pub fn death_animation_spawner(
    mut commands: Commands,
    mut reader: MessageReader<GameEvent>,
    query: Query<(
        &Position,
        Option<&MonsterMarker>,
        &game_core::components::MonsterComp,
    )>,
    effect_assets: Option<Res<EffectAssets>>,
) {
    let Some(assets) = effect_assets else { return };

    for ev in reader.read() {
        if let GameEvent::EntityKilled { entity, .. } = ev {
            let Ok((pos, monster_marker, monster_comp)) = query.get(*entity) else {
                continue;
            };
            // 只对怪物播放死亡碎片
            if monster_marker.is_none() {
                continue;
            }

            // 根据 monster_type 奇偶选颜色（与 entity_shapes 一致）
            let mat = if monster_comp.monster_type % 2 == 0 {
                assets.fragment_mat_green.clone()
            } else {
                assets.fragment_mat_purple.clone()
            };

            // spawn 4 个碎片，随机方向散开
            for i in 0..4 {
                let angle = std::f32::consts::FRAC_PI_2 * i as f32
                    + (entity.index_u32() as f32 * 1.7).sin() * 0.5;
                let speed = 80.0 + (i as f32 * 20.0);
                commands.spawn((
                    Mesh2d(assets.fragment_mesh.clone()),
                    MeshMaterial2d(mat.clone()),
                    Transform::from_xyz(pos.x, pos.y, 3.0)
                        .with_rotation(Quat::from_rotation_z(angle)),
                    DeathFragment {
                        timer: Timer::from_seconds(0.5, TimerMode::Once),
                        vel_x: angle.cos() * speed,
                        vel_y: angle.sin() * speed,
                    },
                ));
            }
        }
    }
}

/// 死亡碎片：散开 + 缩放淡出
pub fn death_fragment_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DeathFragment, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut frag, mut transform) in &mut query {
        frag.timer.tick(time.delta());
        let t = frag.timer.fraction();

        // 位移
        transform.translation.x += frag.vel_x * dt;
        transform.translation.y += frag.vel_y * dt;

        // 缩放淡出（从 1.0 缩到 0.0）
        let scale = 1.0 - t;
        transform.scale = Vec3::splat(scale);

        if frag.timer.is_finished() {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
        }
    }
}

/// 掉落物旋转 + 呼吸发光动画
pub fn loot_animation_system(time: Res<Time>, mut query: Query<(&LootMarker, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (loot, mut transform) in &mut query {
        let phase = t + loot.phase_offset;
        transform.rotation = Quat::from_rotation_z(phase * 1.5);
        let breath = phase.sin() * 0.3 + 0.7;
        transform.scale = Vec3::new(breath, breath, 1.0);
    }
}

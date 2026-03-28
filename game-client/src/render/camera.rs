use bevy::prelude::*;

use game_core::components::Position;

use crate::sync::LocalPlayerMarker;

/// 游戏摄像机标记
#[derive(Component)]
pub struct GameCamera;

/// 摄像机跟随平滑系数
const CAMERA_LERP_SPEED: f32 = 8.0;

/// 初始化摄像机
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        GameCamera,
        // z=100 确保所有游戏对象（z=-100 网格 到 z=10 特效）在 near/far 裁剪范围内
        // OrthographicProjection::default_2d() near=-1000, far=1000
        // 可见 Z 范围: [100-1000, 100+1000] = [-900, 1100]
        Transform::from_translation(Vec3::new(0.0, 0.0, 100.0)),
    ));
}

/// 摄像机 lerp 跟随本地玩家
pub fn camera_follow_system(
    time: Res<Time>,
    player_query: Query<&Position, With<LocalPlayerMarker>>,
    mut camera_query: Query<&mut Transform, With<GameCamera>>,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };
    let Ok(mut cam_transform) = camera_query.single_mut() else {
        return;
    };

    let target = Vec3::new(player_pos.x, player_pos.y, cam_transform.translation.z);
    let dt = time.delta_secs();
    let t = (CAMERA_LERP_SPEED * dt).min(1.0);
    cam_transform.translation = cam_transform.translation.lerp(target, t);
}

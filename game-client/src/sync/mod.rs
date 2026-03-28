//! 同步模块：本地标记组件 + HTTP 同步管理器

pub mod http_sync;

use bevy::prelude::*;

use game_core::meta::save_data::SaveData;
use game_core::meta::state::MetaState;

use http_sync::{PullPending, SyncManager, UploadPending};

// ─── 标记组件 ───

/// 标记本地玩家实体
#[derive(Component)]
pub struct LocalPlayerMarker;

// ─── SyncPlugin ───

pub struct SyncPlugin;

/// 后台同步间隔（秒）
const SYNC_INTERVAL: f32 = 300.0;

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SyncManager>()
            .init_resource::<PullPending>()
            .init_resource::<UploadPending>()
            .add_systems(Startup, startup_pull)
            .add_systems(
                Update,
                (poll_pull_result, poll_upload_result, background_sync_timer),
            );
    }
}

/// 6.2.1 启动时发起 pullState()
fn startup_pull(pull: Res<PullPending>) {
    http_sync::start_pull(&pull);
}

/// 轮询 pull 结果，用服务端数据覆盖本地 MetaState
fn poll_pull_result(
    pull: Res<PullPending>,
    meta: Option<ResMut<MetaState>>,
    mut sync_mgr: ResMut<SyncManager>,
) {
    if let Some(bytes) = pull.0.take() {
        if !bytes.is_empty() {
            if let Some(mut meta) = meta {
                let config = bincode::config::standard();
                if let Ok((save_data, _)) =
                    bincode::decode_from_slice::<SaveData, _>(&bytes, config)
                {
                    apply_save_data_to_meta(&save_data, &mut meta);
                }
            }
        }
        sync_mgr.is_syncing = false;
    }
}

/// 轮询 upload 结果
fn poll_upload_result(upload: Res<UploadPending>, mut sync_mgr: ResMut<SyncManager>) {
    if upload.0.take().is_some() {
        sync_mgr.is_syncing = false;
    }
}

/// 6.2.4 后台定时同步（每 300 秒）
fn background_sync_timer(
    time: Res<Time>,
    mut sync_mgr: ResMut<SyncManager>,
    pull: Res<PullPending>,
    upload: Res<UploadPending>,
) {
    if sync_mgr.is_syncing {
        return;
    }

    let elapsed = time.elapsed_secs();
    if elapsed - sync_mgr.last_sync_time >= SYNC_INTERVAL {
        sync_mgr.last_sync_time = elapsed;
        sync_mgr.is_syncing = true;
        http_sync::start_upload_pending(&upload);
        http_sync::start_pull(&pull);
    }
}

/// 将 SaveData 应用到 MetaState
fn apply_save_data_to_meta(save: &SaveData, meta: &mut MetaState) {
    // 合并解锁的 program types（服务端权威，取并集）
    for &type_id in &save.meta_unlocks.unlocked_program_types {
        if let Some(pt) = program_type_from_id(type_id) {
            meta.unlocked_program_types.insert(pt);
        }
    }

    // 合并解锁的 disc qualities
    for &quality_id in &save.meta_unlocks.unlocked_disc_qualities {
        if let Some(dq) = disc_quality_from_id(quality_id) {
            meta.unlocked_disc_qualities.insert(dq);
        }
    }

    // 合并里程碑进度（取较大值）
    for &(id, progress) in &save.milestone_progress.entries {
        let entry = meta.milestone_progress.entry(id).or_insert(0.0);
        if progress > *entry {
            *entry = progress;
        }
    }
}

fn program_type_from_id(id: u32) -> Option<game_core::data::program::ProgramType> {
    use game_core::data::program::ProgramType;
    match id {
        0 => Some(ProgramType::Projectile),
        1 => Some(ProgramType::TrajectoryMod),
        2 => Some(ProgramType::QuantityMod),
        3 => Some(ProgramType::Trigger),
        4 => Some(ProgramType::StatusEffect),
        5 => Some(ProgramType::FormationMod),
        6 => Some(ProgramType::TimingMod),
        7 => Some(ProgramType::DamageMod),
        8 => Some(ProgramType::ProjectileMod),
        9 => Some(ProgramType::FieldMod),
        _ => None,
    }
}

fn disc_quality_from_id(id: u8) -> Option<game_core::data::disc::DiscQuality> {
    use game_core::data::disc::DiscQuality;
    match id {
        0 => Some(DiscQuality::Common),
        1 => Some(DiscQuality::Uncommon),
        2 => Some(DiscQuality::Rare),
        3 => Some(DiscQuality::Epic),
        4 => Some(DiscQuality::Legendary),
        _ => None,
    }
}

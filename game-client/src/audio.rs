//! 音效系统：射击、命中、死亡、拾取、升级
//!
//! WASM 环境下 Bevy Audio 通过 Web Audio API 工作，
//! 需要用户交互（点击）后才能播放音效（浏览器 autoplay policy）。

use bevy::prelude::*;

// GameState 将在添加播放 system 时使用
#[allow(unused_imports)]
use crate::GameState;

/// 音效资源 handles，启动时加载
#[derive(Resource, Default)]
pub struct GameAudio {
    // 后续加载具体音效文件（.ogg 格式，WASM 兼容性最好）
    // pub shoot: Handle<AudioSource>,
    // pub hit: Handle<AudioSource>,
    // pub death: Handle<AudioSource>,
    // pub pickup: Handle<AudioSource>,
    // pub level_up: Handle<AudioSource>,
}

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAudio>();
        // 后续添加音效加载和播放 system：
        // .add_systems(Startup, load_audio_assets)
        // .add_systems(Update, (
        //     shoot_sound_system,
        //     hit_sound_system,
        //     death_sound_system,
        //     pickup_sound_system,
        //     level_up_sound_system,
        // ).run_if(in_state(GameState::Playing)))
    }
}

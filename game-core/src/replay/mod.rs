//! 输入录像与回放数据结构

use bincode::{Decode, Encode};

use crate::events::RunResult;

/// 单帧输入快照
#[derive(Clone, Debug, Encode, Decode)]
pub struct InputFrame {
    pub tick_number: u32,
    pub move_x: f32,
    pub move_y: f32,
    pub aim_x: f32,
    pub aim_y: f32,
}

/// 完整录像数据
#[derive(Clone, Debug, Encode, Decode)]
pub struct ReplayData {
    pub version: u32,
    pub season_id: u32,
    pub seed: u64,
    pub start_chunk: (i32, i32),
    pub frames: Vec<InputFrame>,
    pub result: Option<RunResult>,
}

impl ReplayData {
    pub fn new(season_id: u32, seed: u64, start_chunk: (i32, i32)) -> Self {
        Self {
            version: 1,
            season_id,
            seed,
            start_chunk,
            frames: Vec::new(),
            result: None,
        }
    }

    pub fn push_frame(&mut self, frame: InputFrame) {
        self.frames.push(frame);
    }
}

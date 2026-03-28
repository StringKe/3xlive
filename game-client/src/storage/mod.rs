//! IndexedDB 存储层
//!
//! 异步桥接方案：JS 回调通过 wasm_bindgen_futures::spawn_local 发起 async 操作，
//! 结果写入 Rc<RefCell<Option<T>>> 共享状态，Bevy System 每帧 poll 检查。

use std::cell::RefCell;
use std::rc::Rc;

use bevy::prelude::*;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::js_bridge;

/// 异步操作的共享状态容器
#[derive(Resource)]
pub struct StoragePending<T: Send + Sync + 'static> {
    inner: Rc<RefCell<Option<T>>>,
}

impl<T: Send + Sync + 'static> Default for StoragePending<T> {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(None)),
        }
    }
}

impl<T: Send + Sync + 'static> StoragePending<T> {
    /// 取出结果（如果已就绪），取出后清空
    pub fn take(&self) -> Option<T> {
        self.inner.borrow_mut().take()
    }

    /// 获取共享 Rc 用于 spawn_local 闭包
    pub fn shared(&self) -> Rc<RefCell<Option<T>>> {
        Rc::clone(&self.inner)
    }
}

/// 从 IndexedDB 异步加载存档
pub fn load_save_data(pending: &StoragePending<Option<Vec<u8>>>) {
    let slot = pending.shared();
    spawn_local(async move {
        let result = js_bridge::storage_get("save_data", "current").await;
        let data = if result.is_null() || result.is_undefined() {
            None
        } else {
            let arr: Uint8Array = result.unchecked_into();
            Some(arr.to_vec())
        };
        *slot.borrow_mut() = Some(data);
    });
}

/// 异步写入存档到 IndexedDB
pub fn write_save_data(data: &[u8]) {
    let owned = data.to_vec();
    spawn_local(async move {
        js_bridge::storage_put("save_data", "current", &owned).await;
    });
}

/// 异步写入 pending_upload 记录
pub fn write_pending_upload(key: &str, data: &[u8]) {
    let key = key.to_string();
    let owned = data.to_vec();
    spawn_local(async move {
        js_bridge::storage_put("pending_uploads", &key, &owned).await;
    });
}

/// 异步写入录像到 replay_archive（FIFO 保留最近 10 局）
pub fn write_replay(key: &str, data: &[u8]) {
    let key = key.to_string();
    let owned = data.to_vec();
    spawn_local(async move {
        // 写入新录像
        js_bridge::storage_put("replay_archive", &key, &owned).await;

        // FIFO 清理：获取所有 key，按字典序排序，删除超出 10 个的旧记录
        let keys_js = js_bridge::storage_get_all_keys("replay_archive").await;
        if let Ok(arr) = keys_js.dyn_into::<js_sys::Array>() {
            let mut keys: Vec<String> = arr.iter().filter_map(|v| v.as_string()).collect();
            keys.sort();
            while keys.len() > 10 {
                if let Some(oldest) = keys.first() {
                    js_bridge::storage_del("replay_archive", oldest).await;
                }
                keys.remove(0);
            }
        }
    });
}

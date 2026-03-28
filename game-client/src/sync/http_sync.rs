//! HTTP 同步管理器
//!
//! SyncManager Resource 负责调度 HTTP 同步操作。
//! 所有 HTTP 请求通过 spawn_local 发起，结果通过 WasmShared 回传。

use std::cell::RefCell;
use std::rc::Rc;

use bevy::prelude::*;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::js_bridge;

/// WASM 单线程环境下的共享状态容器
pub struct WasmShared<T>(Rc<RefCell<Option<T>>>);

// SAFETY: WASM 是单线程环境，不存在跨线程访问
unsafe impl<T> Send for WasmShared<T> {}
unsafe impl<T> Sync for WasmShared<T> {}

impl<T> Default for WasmShared<T> {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(None)))
    }
}

impl<T> WasmShared<T> {
    /// 取出结果（如果已就绪），取出后清空
    pub fn take(&self) -> Option<T> {
        self.0.borrow_mut().take()
    }

    /// 获取共享 Rc 用于 spawn_local 闭包
    pub fn shared(&self) -> Rc<RefCell<Option<T>>> {
        Rc::clone(&self.0)
    }
}

/// 同步管理器 Resource
#[derive(Resource)]
pub struct SyncManager {
    /// 上次同步时间（秒，来自 Time::elapsed_secs）
    pub last_sync_time: f32,
    /// 是否有正在进行的同步操作
    pub is_syncing: bool,
}

impl Default for SyncManager {
    fn default() -> Self {
        Self {
            last_sync_time: 0.0,
            is_syncing: false,
        }
    }
}

/// Pull 结果：服务端返回的 meta 数据（bincode 编码的字节）
#[derive(Resource, Default)]
pub struct PullPending(pub WasmShared<Vec<u8>>);

/// Upload 完成标记
#[derive(Resource, Default)]
pub struct UploadPending(pub WasmShared<bool>);

/// 发起 pullState() 请求
pub fn start_pull(pending: &PullPending) {
    let slot = pending.0.shared();
    spawn_local(async move {
        let result = js_bridge::sync_pull_state().await;
        let data = if result.is_null() || result.is_undefined() {
            vec![]
        } else {
            let arr: Uint8Array = result.unchecked_into();
            arr.to_vec()
        };
        *slot.borrow_mut() = Some(data);
    });
}

/// 收集所有 pending_uploads 并发起上传，完成后设置标记
pub fn start_upload_pending(pending: &UploadPending) {
    let slot = pending.0.shared();
    spawn_local(async move {
        // 从 IndexedDB 读取所有 pending_uploads
        let keys_js = js_bridge::storage_get_all_keys("pending_uploads").await;
        let keys: Vec<String> = if let Ok(arr) = keys_js.dyn_into::<js_sys::Array>() {
            arr.iter().filter_map(|v| v.as_string()).collect()
        } else {
            vec![]
        };

        for key in &keys {
            let data_js = js_bridge::storage_get("pending_uploads", key).await;
            if !data_js.is_null() && !data_js.is_undefined() {
                let arr: Uint8Array = data_js.unchecked_into();
                let bytes = arr.to_vec();
                let _resp = js_bridge::sync_upload_results(&bytes).await;
                // 上传成功后删除
                js_bridge::storage_del("pending_uploads", key).await;
            }
        }

        *slot.borrow_mut() = Some(true);
    });
}

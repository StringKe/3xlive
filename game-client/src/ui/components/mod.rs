//! UI 基础组件库：可复用的 egui 组件集合

pub mod button;
pub mod card;
pub mod container;
pub mod divider;
pub mod group;
pub mod overlay;
pub mod progress;
pub mod stack;
pub mod text;
pub mod tooltip;

// 重导出所有组件的公开 API，供外部模块统一引入
#[allow(unused_imports)]
pub use button::{button, ButtonProps, ButtonVariant};
#[allow(unused_imports)]
pub use card::card;
#[allow(unused_imports)]
pub use container::container;
#[allow(unused_imports)]
pub use divider::{divider, DividerDirection};
#[allow(unused_imports)]
pub use group::group;
#[allow(unused_imports)]
pub use overlay::{overlay, OverlayAnchor};
#[allow(unused_imports)]
pub use progress::{progress, ProgressProps};
#[allow(unused_imports)]
pub use stack::stack;
#[allow(unused_imports)]
pub use text::{text, TextProps, TextSize, TextWeight};
#[allow(unused_imports)]
pub use tooltip::tooltip_on_hover;

/// 响应式尺寸单位（类似 CSS rem）
///
/// 基准：1440px 宽屏幕 → base=14.4，375px 手机 → base=4.0
/// 用法：`rem(screen_w, 4.5)` → 标题字号（1440px→64.8, 375px→18.0）
pub fn rem(screen_w: f32, n: f32) -> f32 {
    (screen_w / 100.0).clamp(4.0, 14.4) * n
}

/// i18n 翻译 helper
///
/// WASM 构建时通过 js_bridge 调用浏览器端翻译；
/// native 构建时直接返回 key 本身（用于测试/开发）。
#[cfg(target_arch = "wasm32")]
pub fn t(key: &str) -> String {
    crate::js_bridge::t(key)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn t(key: &str) -> String {
    key.to_string()
}

//! egui 初始化：插件注册、全局样式、颜色工具、字体加载

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use std::cell::RefCell;

// ─── 颜色常量（匹配 Pencil 设计变量） ───

pub struct EguiColors;
impl EguiColors {
    // 主色调
    pub const CYAN: egui::Color32 = egui::Color32::from_rgb(0, 228, 255);
    pub const CYAN_DIM: egui::Color32 = egui::Color32::from_rgb(34, 211, 238);

    // 背景
    pub const BG_DEEP: egui::Color32 = egui::Color32::from_rgb(10, 15, 26);
    pub const BG_INSET: egui::Color32 = egui::Color32::from_rgb(15, 23, 42);
    pub const BG_CARD: egui::Color32 = egui::Color32::from_rgb(30, 41, 59);
    pub const PANEL_BG: egui::Color32 = egui::Color32::from_rgba_premultiplied(10, 15, 26, 242);

    // 文字
    pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
    pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(148, 163, 184);
    pub const TEXT_TERTIARY: egui::Color32 = egui::Color32::from_rgb(100, 116, 139);
    pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(71, 85, 105);
    pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(102, 102, 112);
    pub const TEXT_LIGHT: egui::Color32 = egui::Color32::from_rgb(224, 224, 224);

    // 边框
    pub const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(26, 42, 58);
    pub const BORDER_CYAN: egui::Color32 = egui::Color32::from_rgb(0, 180, 200);

    // 按钮填充（premultiplied：手动将 unmultiplied RGBA 换算为 r*a/255 等）
    /// 主按钮填充：极淡青色调（alpha ~6%）— unmultiplied(0,228,255,15)
    pub const BTN_FILL: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 13, 15, 15);
    /// 按钮 hover 填充 — unmultiplied(0,228,255,35)
    pub const BTN_FILL_HOVER: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 31, 35, 35);
    /// 次要按钮填充：深色 — unmultiplied(15,23,42,200)
    pub const BTN_FILL_SECONDARY: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(12, 18, 33, 200);

    // 状态颜色
    pub const COLOR_HP: egui::Color32 = egui::Color32::from_rgb(255, 68, 68);
    pub const COLOR_XP: egui::Color32 = egui::Color32::from_rgb(255, 215, 0);
    pub const COLOR_ENEMY: egui::Color32 = egui::Color32::from_rgb(255, 68, 68);
    pub const COLOR_ALLY: egui::Color32 = egui::Color32::from_rgb(68, 136, 255);
    pub const COLOR_SUCCESS: egui::Color32 = egui::Color32::from_rgb(0, 255, 136);

    // 槽位
    pub const EMPTY_SLOT: egui::Color32 = egui::Color32::from_rgb(38, 38, 51);
    /// unmultiplied(71,85,105,51) → premul(14,17,21,51)
    pub const SLOT_BORDER_DIM: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(14, 17, 21, 51);
    pub const SLOT_LOCKED_BG: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(15, 23, 42, 136);

    // 系统消息颜色
    pub const CHAT_SYSTEM: egui::Color32 = egui::Color32::from_rgb(255, 215, 0);

    // 通知
    pub const NOTIFICATION_GOLD: egui::Color32 = egui::Color32::from_rgb(255, 230, 100);

    // Program 槽位 — unmultiplied(0,255,136,102) → premul(0,102,54,102)
    pub const PROGRAM_SLOT_ACTIVE: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(0, 102, 54, 102);

    // 暗色填充（空 buff 槽位等）
    pub const BG_DARK: egui::Color32 = egui::Color32::from_rgb(10, 22, 40);

    // 半透明卡片背景（小地图等）
    pub const CARD_BG_ALPHA: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(15, 23, 42, 204);

    // Cyan 低透明度（debug 边框、分隔线）— unmultiplied(0,228,255,α)
    pub const CYAN_FAINT: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 30, 34, 34);
    pub const CYAN_GHOST: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 19, 21, 21);

    // ── Disc 品质颜色 ──
    pub const QUALITY_COMMON: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
    pub const QUALITY_UNCOMMON: egui::Color32 = egui::Color32::from_rgb(77, 255, 77);
    pub const QUALITY_RARE: egui::Color32 = egui::Color32::from_rgb(77, 128, 255);
    pub const QUALITY_EPIC: egui::Color32 = egui::Color32::from_rgb(200, 77, 255);
    pub const QUALITY_LEGENDARY: egui::Color32 = egui::Color32::from_rgb(255, 200, 50);

    // ── Program 类型颜色 ──
    pub const PROGRAM_PROJECTILE: egui::Color32 = egui::Color32::from_rgb(255, 180, 50);
    pub const PROGRAM_TRAJECTORY: egui::Color32 = egui::Color32::from_rgb(77, 128, 255);
    pub const PROGRAM_QUANTITY: egui::Color32 = egui::Color32::from_rgb(77, 128, 255);
    pub const PROGRAM_TRIGGER: egui::Color32 = egui::Color32::from_rgb(255, 77, 77);
    pub const PROGRAM_STATUS: egui::Color32 = egui::Color32::from_rgb(77, 255, 77);
    pub const PROGRAM_FORMATION: egui::Color32 = egui::Color32::from_rgb(0, 204, 204);
    pub const PROGRAM_TIMING: egui::Color32 = egui::Color32::from_rgb(255, 153, 0);
    pub const PROGRAM_DAMAGE_MOD: egui::Color32 = egui::Color32::from_rgb(255, 51, 51);
    pub const PROGRAM_PROJECTILE_MOD: egui::Color32 = egui::Color32::from_rgb(51, 255, 51);
    pub const PROGRAM_FIELD: egui::Color32 = egui::Color32::from_rgb(204, 0, 204);

    // ── Buff 类型颜色（80% alpha=204, premul = r*204/255） ──
    pub const BUFF_MAX_HEALTH: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(163, 41, 41, 204);
    pub const BUFF_MOVE_SPEED: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(41, 163, 41, 204);
    pub const BUFF_DAMAGE_BOOST: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(184, 102, 21, 204);
    pub const BUFF_EXTRA_SLOT: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(122, 82, 163, 204);
    pub const BUFF_FIRE_RATE: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(184, 184, 41, 204);
    pub const BUFF_SPREAD_REDUCTION: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(62, 163, 163, 204);
    pub const BUFF_PICKUP_RADIUS: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(102, 163, 62, 204);
    pub const BUFF_XP_BOOST: egui::Color32 =
        egui::Color32::from_rgba_premultiplied(163, 143, 204, 204);
}

// ─── 字体异步加载 ───

thread_local! {
    static FONT_DATA: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

/// 标记字体已注入 egui
#[derive(Resource)]
pub struct EguiFontLoaded;

/// 标记 egui visuals 已配置
#[derive(Resource)]
struct EguiVisualsConfigured;

// ─── 插件 ───

pub struct EguiSetupPlugin;

impl Plugin for EguiSetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_systems(Startup, start_font_fetch)
            // egui 系统：必须在 EguiPrimaryContextPass 中运行
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                (
                    configure_visuals.run_if(not(resource_exists::<EguiVisualsConfigured>)),
                    inject_egui_font.run_if(not(resource_exists::<EguiFontLoaded>)),
                ),
            );
    }
}

/// 设置 egui 全局深色主题
fn configure_visuals(mut commands: Commands, mut contexts: EguiContexts) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = egui::Color32::from_rgba_premultiplied(8, 12, 22, 240);
    visuals.panel_fill = egui::Color32::from_rgba_premultiplied(5, 5, 13, 230);
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(15, 15, 25);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_premultiplied(0, 50, 60, 40);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_premultiplied(0, 100, 120, 80);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgba_premultiplied(0, 150, 180, 100);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, EguiColors::CYAN);
    visuals.widgets.hovered.fg_stroke =
        egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 240, 255));
    visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
    visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(0, 228, 255, 40);
    visuals.selection.stroke = egui::Stroke::new(1.0, EguiColors::CYAN);
    ctx.set_visuals(visuals);
    commands.insert_resource(EguiVisualsConfigured);
}

/// 发起字体文件异步 fetch
fn start_font_fetch() {
    let font_path = crate::js_bridge::font_path();
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_bytes(&font_path).await {
            Ok(data) => {
                FONT_DATA.with(|f| *f.borrow_mut() = Some(data));
            }
            Err(_e) => {
                crate::dev_warn!("字体加载失败: {_e:?}");
            }
        }
    });
}

/// 检查字体数据是否就绪，注入 egui
fn inject_egui_font(mut commands: Commands, mut contexts: EguiContexts) {
    let data = FONT_DATA.with(|f| f.borrow_mut().take());
    let Some(font_bytes) = data else { return };

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "game_font".to_owned(),
        egui::FontData::from_owned(font_bytes).into(),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "game_font".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "game_font".to_owned());
    ctx.set_fonts(fonts);

    commands.insert_resource(EguiFontLoaded);
}

/// 通过 web_sys fetch API 获取字节数据
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or("no window")?;
    let resp_value = JsFuture::from(window.fetch_with_str(url)).await?;
    let resp: web_sys::Response = resp_value.dyn_into()?;
    if !resp.ok() {
        return Err(format!("HTTP {}: {}", resp.status(), resp.status_text()).into());
    }
    let array_buffer = JsFuture::from(resp.array_buffer()?).await?;
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    Ok(uint8_array.to_vec())
}

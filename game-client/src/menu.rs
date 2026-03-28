//! 主菜单 Plugin：登录、开始游戏入口（egui 版本）

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::arena::ArenaState;
use crate::egui_setup::EguiColors;
use crate::js_bridge;
use crate::menu_panels::{self, MailboxData, MenuPanel, SeasonGoalsData};
use crate::simulation::GameMode;
use crate::sync::http_sync::{PullPending, SyncManager, UploadPending};
use crate::ui::components::button::{button, ButtonProps, ButtonVariant};
use crate::ui::components::rem;
use crate::ui::hud::HudLayout;
use crate::GameState;

/// 游戏字体 Resource，全局共享（Text2d 特效仍需要）
#[derive(Resource)]
pub struct GameFont(pub Handle<Font>);

/// 缓存 auth 版本号，避免每帧都调用 JS 反序列化
#[derive(Resource, Default)]
struct AuthStateVersion(u32);

/// 缓存的 auth 状态
#[derive(Resource, Default)]
struct CachedAuthState {
    is_signed_in: bool,
    display_name: Option<String>,
}

// ─── 插件 ───

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        // 字体必须在 build 阶段插入，OnEnter(Menu) 在第一帧就触发
        let font_path = js_bridge::font_path();
        let font = app.world().resource::<AssetServer>().load(&font_path);
        app.insert_resource(GameFont(font));

        app.init_resource::<AuthStateVersion>()
            .init_resource::<CachedAuthState>()
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                menu_ui_system.run_if(in_state(GameState::Menu)),
            );
    }
}

// ─── menu_ui_system ───

#[allow(clippy::too_many_arguments)]
fn menu_ui_system(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<GameState>>,
    mut auth_version: ResMut<AuthStateVersion>,
    mut cached_auth: ResMut<CachedAuthState>,
    mut sync_mgr: ResMut<SyncManager>,
    pull_pending: Res<PullPending>,
    upload_pending: Res<UploadPending>,
    mut next_arena: ResMut<NextState<ArenaState>>,
    mut menu_panel: ResMut<MenuPanel>,
    mut game_mode: ResMut<GameMode>,
    mailbox: Res<MailboxData>,
    goals: Res<SeasonGoalsData>,
    hud: Res<HudLayout>,
) {
    // 轮询 auth 状态变化
    let current_version = js_bridge::auth_state_version();
    if current_version != auth_version.0 {
        auth_version.0 = current_version;
        if let Some(auth) = read_auth_state() {
            cached_auth.is_signed_in = auth.is_clerk_signed_in;
            cached_auth.display_name = auth.display_name;
        } else {
            cached_auth.is_signed_in = false;
            cached_auth.display_name = None;
        }
    }

    let signed_in = cached_auth.is_signed_in;

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_rgba_premultiplied(8, 13, 23, 242)))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.15);

                // 根据 MenuPanel 状态决定显示内容
                match *menu_panel {
                    MenuPanel::None => {
                        render_main_menu(
                            ui,
                            signed_in,
                            &cached_auth,
                            &mut next_state,
                            &mut sync_mgr,
                            &pull_pending,
                            &upload_pending,
                            &mut next_arena,
                            &mut menu_panel,
                            &mut game_mode,
                            hud.screen_w,
                        );
                    }
                    MenuPanel::Mailbox => {
                        menu_panels::render_mailbox(ui, &mailbox, &mut menu_panel);
                    }
                    MenuPanel::SeasonGoals => {
                        menu_panels::render_season_goals(ui, &goals, &mut menu_panel);
                    }
                }
            });
        });
}

/// 菜单响应式尺寸（从 screen_w 推导）
struct MenuSizes {
    title: f32,
    btn_font: f32,
    btn_w: f32,
    btn_h: f32,
    spacing: f32,
}

impl MenuSizes {
    fn from_screen_w(sw: f32) -> Self {
        Self {
            title: rem(sw, 4.5),
            btn_font: rem(sw, 1.4),
            btn_w: rem(sw, 16.7),
            btn_h: rem(sw, 3.5),
            spacing: rem(sw, 0.8),
        }
    }
}

fn menu_btn(label: &str, screen_w: f32) -> ButtonProps {
    let sizes = MenuSizes::from_screen_w(screen_w);
    ButtonProps {
        label: label.to_string(),
        variant: ButtonVariant::Primary,
        font_size: sizes.btn_font,
        min_width: Some(sizes.btn_w),
        min_height: Some(sizes.btn_h),
        ..Default::default()
    }
}

/// 渲染主菜单内容
#[allow(clippy::too_many_arguments)]
fn render_main_menu(
    ui: &mut egui::Ui,
    signed_in: bool,
    cached_auth: &CachedAuthState,
    next_state: &mut ResMut<NextState<GameState>>,
    sync_mgr: &mut ResMut<SyncManager>,
    pull_pending: &Res<PullPending>,
    upload_pending: &Res<UploadPending>,
    next_arena: &mut ResMut<NextState<ArenaState>>,
    menu_panel: &mut ResMut<MenuPanel>,
    game_mode: &mut ResMut<GameMode>,
    screen_w: f32,
) {
    let sizes = MenuSizes::from_screen_w(screen_w);

    // 标题
    ui.label(
        egui::RichText::new("3X LIVE")
            .size(sizes.title)
            .color(EguiColors::CYAN),
    );
    ui.add_space(sizes.spacing * 3.0);

    // Start Run
    if button(ui, &menu_btn(&js_bridge::t("ui.menu.start_game"), screen_w)) {
        **game_mode = GameMode::Normal;
        next_state.set(GameState::Playing);
    }
    ui.add_space(sizes.spacing);

    // Training Ground
    if button(
        ui,
        &ButtonProps {
            label: js_bridge::t("ui.lifecycle.training"),
            variant: ButtonVariant::Secondary,
            font_size: (sizes.btn_font * 0.9).max(12.0),
            min_width: Some(sizes.btn_w),
            min_height: Some(sizes.btn_h * 0.88),
            ..Default::default()
        },
    ) {
        **game_mode = GameMode::Training;
        next_state.set(GameState::Playing);
    }
    ui.add_space(sizes.spacing);

    // Arena
    if button(ui, &menu_btn(&js_bridge::t("ui.menu.arena"), screen_w)) {
        next_arena.set(ArenaState::Building);
    }
    ui.add_space(sizes.spacing);

    // Sync
    {
        let sync_label = if sync_mgr.is_syncing {
            js_bridge::t("ui.menu.syncing")
        } else {
            js_bridge::t("ui.menu.sync")
        };
        if button(
            ui,
            &ButtonProps {
                enabled: !sync_mgr.is_syncing,
                ..menu_btn(&sync_label, screen_w)
            },
        ) {
            sync_mgr.is_syncing = true;
            crate::sync::http_sync::start_upload_pending(upload_pending);
            crate::sync::http_sync::start_pull(pull_pending);
        }
    }
    ui.add_space(sizes.spacing);

    // Mailbox
    if button(ui, &menu_btn(&js_bridge::t("ui.menu.mailbox"), screen_w)) {
        **menu_panel = MenuPanel::Mailbox;
    }
    ui.add_space(sizes.spacing);

    // Goals
    if button(ui, &menu_btn(&js_bridge::t("ui.menu.goals"), screen_w)) {
        **menu_panel = MenuPanel::SeasonGoals;
    }
    ui.add_space(sizes.spacing);

    if signed_in {
        if let Some(ref name) = cached_auth.display_name {
            ui.label(
                egui::RichText::new(name.as_str())
                    .size(sizes.btn_font * 0.9)
                    .color(EguiColors::TEXT_LIGHT),
            );
            ui.add_space(sizes.spacing);
        }

        if button(ui, &menu_btn(&js_bridge::t("ui.menu.profile"), screen_w)) {
            js_bridge::open_user_profile();
        }
        ui.add_space(sizes.spacing);

        if button(ui, &menu_btn(&js_bridge::t("ui.menu.sign_out"), screen_w)) {
            wasm_bindgen_futures::spawn_local(async {
                js_bridge::sign_out().await;
            });
        }
    } else if button(ui, &menu_btn(&js_bridge::t("ui.menu.sign_in"), screen_w)) {
        js_bridge::open_sign_in();
    }
}

// ─── 辅助函数 ───

fn read_auth_state() -> Option<js_bridge::AuthState> {
    let val = js_bridge::get_auth_state();
    serde_wasm_bindgen::from_value(val).ok()
}

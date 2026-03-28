//! 主菜单子面板：邮箱、赛季目标
//!
//! 每个面板通过 MenuPanel Resource 切换显示。
//! 面板内容在主菜单 CentralPanel 内部渲染（非浮窗）。

use bevy::prelude::*;
use bevy_egui::egui;

use crate::egui_setup::EguiColors;
use crate::js_bridge;
use crate::ui::components::button::{button, ButtonProps, ButtonVariant};

/// 当前打开的菜单面板
#[derive(Resource, Default, PartialEq, Eq)]
pub enum MenuPanel {
    #[default]
    None,
    Mailbox,
    SeasonGoals,
}

/// 邮件条目（从 pullState 解析）
#[derive(Resource, Default)]
pub struct MailboxData {
    pub mails: Vec<MailEntry>,
}

pub struct MailEntry {
    pub id: u32,
    pub from_name: String,
    pub item_type: String,
    pub claimed: bool,
}

/// 赛季目标数据
#[derive(Resource, Default)]
pub struct SeasonGoalsData {
    pub goals: Vec<GoalEntry>,
}

pub struct GoalEntry {
    pub name: String,
    pub current: f64,
    pub target: f64,
}

pub struct MenuPanelsPlugin;

impl Plugin for MenuPanelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuPanel>()
            .init_resource::<MailboxData>()
            .init_resource::<SeasonGoalsData>();
    }
}

/// 面板标题 + 返回按钮
fn panel_header(ui: &mut egui::Ui, title: &str, panel: &mut MenuPanel) {
    ui.label(
        egui::RichText::new(title)
            .size(36.0)
            .color(EguiColors::CYAN),
    );
    ui.add_space(24.0);

    if button(
        ui,
        &ButtonProps {
            label: js_bridge::t("ui.menu.back"),
            variant: ButtonVariant::Primary,
            font_size: 16.0,
            min_width: Some(160.0),
            min_height: Some(40.0),
            ..Default::default()
        },
    ) {
        *panel = MenuPanel::None;
    }
    ui.add_space(24.0);
}

/// 渲染邮箱面板
pub fn render_mailbox(ui: &mut egui::Ui, mailbox: &MailboxData, panel: &mut MenuPanel) {
    panel_header(ui, &js_bridge::t("ui.menu.mailbox"), panel);

    if mailbox.mails.is_empty() {
        ui.label(
            egui::RichText::new(js_bridge::t("ui.mail.no_mail")).color(EguiColors::TEXT_SECONDARY),
        );
    } else {
        for mail in &mailbox.mails {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} | {}", mail.from_name, mail.item_type))
                        .color(EguiColors::TEXT_LIGHT),
                );
                if !mail.claimed {
                    if button(
                        ui,
                        &ButtonProps {
                            label: js_bridge::t("ui.mail.claim"),
                            variant: ButtonVariant::Primary,
                            font_size: 14.0,
                            ..Default::default()
                        },
                    ) {
                        let mail_id = mail.id;
                        wasm_bindgen_futures::spawn_local(async move {
                            let _ = crate::js_bridge::sync_claim_mail(mail_id as f64).await;
                        });
                    }
                } else {
                    ui.label(
                        egui::RichText::new(js_bridge::t("ui.mail.claimed"))
                            .color(EguiColors::TEXT_MUTED),
                    );
                }
            });
            ui.add_space(4.0);
        }
    }
}

/// 渲染赛季目标面板
pub fn render_season_goals(ui: &mut egui::Ui, goals: &SeasonGoalsData, panel: &mut MenuPanel) {
    panel_header(ui, &js_bridge::t("ui.season.goals_title"), panel);

    if goals.goals.is_empty() {
        ui.label(
            egui::RichText::new(js_bridge::t("ui.season.no_season"))
                .color(EguiColors::TEXT_SECONDARY),
        );
    } else {
        for goal in &goals.goals {
            ui.label(
                egui::RichText::new(&goal.name)
                    .size(14.0)
                    .color(EguiColors::CYAN),
            );
            let progress = (goal.current / goal.target).min(1.0) as f32;
            ui.add(
                egui::ProgressBar::new(progress)
                    .text(format!("{:.0} / {:.0}", goal.current, goal.target))
                    .fill(EguiColors::CYAN),
            );

            if goal.current >= goal.target {
                ui.label(
                    egui::RichText::new(js_bridge::t("ui.season.goal_complete"))
                        .color(EguiColors::QUALITY_LEGENDARY),
                );
            }
            ui.add_space(8.0);
        }
    }
}


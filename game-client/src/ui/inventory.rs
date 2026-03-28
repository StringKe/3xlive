//! 背包工具函数：颜色、图标、tooltip 格式化
//! 渲染逻辑已移入 hud/bottom_panel.rs

use bevy_egui::egui;

use super::{ClientItem, ClientProgram};
use crate::egui_setup::EguiColors;
#[cfg(target_arch = "wasm32")]
use crate::js_bridge;
use game_core::data::disc::DiscQuality;
use game_core::data::program::{
    DamageModType, FieldType, FormationType, ProgramParams, ProgramType, ProjectileModType,
    StatusEffectType, TimingType, TrajectoryType, TriggerType,
};

/// i18n helper：WASM 环境调用 js_bridge::t，native 编译（测试）时返回 key 本身
fn t(key: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        js_bridge::t(key)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        key.to_string()
    }
}

/// Disc 品质 → i18n key
fn quality_i18n_key(q: &DiscQuality) -> &'static str {
    match q {
        DiscQuality::Common => "ui.quality.common",
        DiscQuality::Uncommon => "ui.quality.uncommon",
        DiscQuality::Rare => "ui.quality.rare",
        DiscQuality::Epic => "ui.quality.epic",
        DiscQuality::Legendary => "ui.quality.legendary",
    }
}

/// Program 类型 → i18n key（完整名）
fn program_type_i18n_key(pt: ProgramType) -> &'static str {
    match pt {
        ProgramType::Projectile => "ui.program_type.projectile",
        ProgramType::TrajectoryMod => "ui.program_type.trajectory_mod",
        ProgramType::QuantityMod => "ui.program_type.quantity_mod",
        ProgramType::Trigger => "ui.program_type.trigger",
        ProgramType::StatusEffect => "ui.program_type.status_effect",
        ProgramType::FormationMod => "ui.program_type.formation_mod",
        ProgramType::TimingMod => "ui.program_type.timing_mod",
        ProgramType::DamageMod => "ui.program_type.damage_mod",
        ProgramType::ProjectileMod => "ui.program_type.projectile_mod",
        ProgramType::FieldMod => "ui.program_type.field_mod",
    }
}

fn trajectory_i18n_key(t: TrajectoryType) -> &'static str {
    match t {
        TrajectoryType::Homing => "ui.trajectory.homing",
        TrajectoryType::Orbit => "ui.trajectory.orbit",
        TrajectoryType::Spiral => "ui.trajectory.spiral",
        TrajectoryType::Wave => "ui.trajectory.wave",
        TrajectoryType::Boomerang => "ui.trajectory.boomerang",
        TrajectoryType::Ricochet => "ui.trajectory.ricochet",
        TrajectoryType::Zigzag => "ui.trajectory.zigzag",
        TrajectoryType::Accelerate => "ui.trajectory.accelerate",
        TrajectoryType::Decelerate => "ui.trajectory.decelerate",
        TrajectoryType::Gravity => "ui.trajectory.gravity",
        TrajectoryType::Scatter => "ui.trajectory.scatter",
        TrajectoryType::Teleport => "ui.trajectory.teleport",
        TrajectoryType::OrbitTarget => "ui.trajectory.orbit_target",
        TrajectoryType::SineVertical => "ui.trajectory.sine_vertical",
        TrajectoryType::Pendulum => "ui.trajectory.pendulum",
    }
}

fn trigger_i18n_key(t: TriggerType) -> &'static str {
    match t {
        TriggerType::OnHit => "ui.trigger.on_hit",
        TriggerType::OnKill => "ui.trigger.on_kill",
        TriggerType::OnExpire => "ui.trigger.on_expire",
        TriggerType::OnPierce => "ui.trigger.on_pierce",
        TriggerType::OnCrit => "ui.trigger.on_crit",
        TriggerType::OnBossHit => "ui.trigger.on_boss_hit",
        TriggerType::OnLowHp => "ui.trigger.on_low_hp",
        TriggerType::Periodic => "ui.trigger.periodic",
        TriggerType::OnDodge => "ui.trigger.on_dodge",
        TriggerType::OnStatusApply => "ui.trigger.on_status_apply",
        TriggerType::OnMultiKill => "ui.trigger.on_multi_kill",
        TriggerType::OnFullHp => "ui.trigger.on_full_hp",
    }
}

fn status_i18n_key(t: StatusEffectType) -> &'static str {
    match t {
        StatusEffectType::Poison => "ui.status.poison",
        StatusEffectType::Freeze => "ui.status.freeze",
        StatusEffectType::Burn => "ui.status.burn",
        StatusEffectType::Slow => "ui.status.slow",
        StatusEffectType::Weaken => "ui.status.weaken",
        StatusEffectType::Shock => "ui.status.shock",
        StatusEffectType::Bleed => "ui.status.bleed",
        StatusEffectType::Curse => "ui.status.curse",
        StatusEffectType::Mark => "ui.status.mark",
        StatusEffectType::Fear => "ui.status.fear",
        StatusEffectType::Corrode => "ui.status.corrode",
        StatusEffectType::Confuse => "ui.status.confuse",
        StatusEffectType::Blind => "ui.status.blind",
        StatusEffectType::GravityWell => "ui.status.gravity_well",
        StatusEffectType::Doom => "ui.status.doom",
    }
}

fn formation_i18n_key(f: FormationType) -> &'static str {
    match f {
        FormationType::Fan => "ui.formation.fan",
        FormationType::Parallel => "ui.formation.parallel",
        FormationType::Cross => "ui.formation.cross",
        FormationType::Star => "ui.formation.star",
        FormationType::Ring => "ui.formation.ring",
        FormationType::Wall => "ui.formation.wall",
        FormationType::Cone => "ui.formation.cone",
        FormationType::Random => "ui.formation.random",
        FormationType::SpiralForm => "ui.formation.spiral_form",
        FormationType::Backshot => "ui.formation.backshot",
    }
}

fn timing_i18n_key(t: TimingType) -> &'static str {
    match t {
        TimingType::Burst => "ui.timing.burst",
        TimingType::Salvo => "ui.timing.salvo",
        TimingType::Delayed => "ui.timing.delayed",
        TimingType::Alternating => "ui.timing.alternating",
        TimingType::Charge => "ui.timing.charge",
        TimingType::Gatling => "ui.timing.gatling",
        TimingType::Heartbeat => "ui.timing.heartbeat",
        TimingType::DoubleTap => "ui.timing.double_tap",
    }
}

fn damage_mod_i18n_key(t: DamageModType) -> &'static str {
    match t {
        DamageModType::Lifesteal => "ui.damage_mod.lifesteal",
        DamageModType::ArmorPen => "ui.damage_mod.armor_pen",
        DamageModType::BonusVsElite => "ui.damage_mod.bonus_vs_elite",
        DamageModType::BonusVsBoss => "ui.damage_mod.bonus_vs_boss",
        DamageModType::Execute => "ui.damage_mod.execute",
        DamageModType::FlatDamage => "ui.damage_mod.flat_damage",
        DamageModType::DamagePercent => "ui.damage_mod.damage_percent",
        DamageModType::CritBoost => "ui.damage_mod.crit_boost",
        DamageModType::CritDamage => "ui.damage_mod.crit_damage",
        DamageModType::Overkill => "ui.damage_mod.overkill",
        DamageModType::Ramp => "ui.damage_mod.ramp",
        DamageModType::GlassCannon => "ui.damage_mod.glass_cannon",
    }
}

fn projectile_mod_i18n_key(t: ProjectileModType) -> &'static str {
    match t {
        ProjectileModType::Chain => "ui.projectile_mod.chain",
        ProjectileModType::Split => "ui.projectile_mod.split",
        ProjectileModType::Explode => "ui.projectile_mod.explode",
        ProjectileModType::Return => "ui.projectile_mod.return_mod",
        ProjectileModType::GrowSize => "ui.projectile_mod.grow_size",
        ProjectileModType::ShrinkSize => "ui.projectile_mod.shrink_size",
        ProjectileModType::Fragment => "ui.projectile_mod.fragment",
        ProjectileModType::Bounce => "ui.projectile_mod.bounce",
        ProjectileModType::Ghost => "ui.projectile_mod.ghost",
        ProjectileModType::Attach => "ui.projectile_mod.attach",
        ProjectileModType::Clone => "ui.projectile_mod.clone_mod",
        ProjectileModType::OrbitHit => "ui.projectile_mod.orbit_hit",
    }
}

fn field_i18n_key(f: FieldType) -> &'static str {
    match f {
        FieldType::DamageZone => "ui.field.damage_zone",
        FieldType::SlowField => "ui.field.slow_field",
        FieldType::HealZone => "ui.field.heal_zone",
        FieldType::GravityPull => "ui.field.gravity_pull",
        FieldType::MineField => "ui.field.mine_field",
        FieldType::Shield => "ui.field.shield",
        FieldType::Beacon => "ui.field.beacon",
        FieldType::Vortex => "ui.field.vortex",
    }
}

/// 根据 ProgramParams 获取描述 i18n key
fn program_desc_key(params: &ProgramParams) -> &'static str {
    match params {
        ProgramParams::Projectile { .. } => "ui.desc.projectile",
        ProgramParams::TrajectoryMod { mod_type } => match mod_type {
            TrajectoryType::Homing => "ui.desc.trajectory.homing",
            TrajectoryType::Orbit => "ui.desc.trajectory.orbit",
            TrajectoryType::Spiral => "ui.desc.trajectory.spiral",
            TrajectoryType::Wave => "ui.desc.trajectory.wave",
            TrajectoryType::Boomerang => "ui.desc.trajectory.boomerang",
            TrajectoryType::Ricochet => "ui.desc.trajectory.ricochet",
            TrajectoryType::Zigzag => "ui.desc.trajectory.zigzag",
            TrajectoryType::Accelerate => "ui.desc.trajectory.accelerate",
            TrajectoryType::Decelerate => "ui.desc.trajectory.decelerate",
            TrajectoryType::Gravity => "ui.desc.trajectory.gravity",
            TrajectoryType::Scatter => "ui.desc.trajectory.scatter",
            TrajectoryType::Teleport => "ui.desc.trajectory.teleport",
            TrajectoryType::OrbitTarget => "ui.desc.trajectory.orbit_target",
            TrajectoryType::SineVertical => "ui.desc.trajectory.sine_vertical",
            TrajectoryType::Pendulum => "ui.desc.trajectory.pendulum",
        },
        ProgramParams::QuantityMod { .. } => "ui.desc.quantity_mod",
        ProgramParams::Trigger { trigger_type, .. } => match trigger_type {
            TriggerType::OnHit => "ui.desc.trigger.on_hit",
            TriggerType::OnKill => "ui.desc.trigger.on_kill",
            TriggerType::OnExpire => "ui.desc.trigger.on_expire",
            TriggerType::OnPierce => "ui.desc.trigger.on_pierce",
            TriggerType::OnCrit => "ui.desc.trigger.on_crit",
            TriggerType::OnBossHit => "ui.desc.trigger.on_boss_hit",
            TriggerType::OnLowHp => "ui.desc.trigger.on_low_hp",
            TriggerType::Periodic => "ui.desc.trigger.periodic",
            TriggerType::OnDodge => "ui.desc.trigger.on_dodge",
            TriggerType::OnStatusApply => "ui.desc.trigger.on_status_apply",
            TriggerType::OnMultiKill => "ui.desc.trigger.on_multi_kill",
            TriggerType::OnFullHp => "ui.desc.trigger.on_full_hp",
        },
        ProgramParams::StatusEffect { effect_type, .. } => match effect_type {
            StatusEffectType::Poison => "ui.desc.status.poison",
            StatusEffectType::Freeze => "ui.desc.status.freeze",
            StatusEffectType::Burn => "ui.desc.status.burn",
            StatusEffectType::Slow => "ui.desc.status.slow",
            StatusEffectType::Weaken => "ui.desc.status.weaken",
            StatusEffectType::Shock => "ui.desc.status.shock",
            StatusEffectType::Bleed => "ui.desc.status.bleed",
            StatusEffectType::Curse => "ui.desc.status.curse",
            StatusEffectType::Mark => "ui.desc.status.mark",
            StatusEffectType::Fear => "ui.desc.status.fear",
            StatusEffectType::Corrode => "ui.desc.status.corrode",
            StatusEffectType::Confuse => "ui.desc.status.confuse",
            StatusEffectType::Blind => "ui.desc.status.blind",
            StatusEffectType::GravityWell => "ui.desc.status.gravity_well",
            StatusEffectType::Doom => "ui.desc.status.doom",
        },
        ProgramParams::FormationMod { formation } => match formation {
            FormationType::Fan => "ui.desc.formation.fan",
            FormationType::Parallel => "ui.desc.formation.parallel",
            FormationType::Cross => "ui.desc.formation.cross",
            FormationType::Star => "ui.desc.formation.star",
            FormationType::Ring => "ui.desc.formation.ring",
            FormationType::Wall => "ui.desc.formation.wall",
            FormationType::Cone => "ui.desc.formation.cone",
            FormationType::Random => "ui.desc.formation.random",
            FormationType::SpiralForm => "ui.desc.formation.spiral_form",
            FormationType::Backshot => "ui.desc.formation.backshot",
        },
        ProgramParams::TimingMod { timing } => match timing {
            TimingType::Burst => "ui.desc.timing.burst",
            TimingType::Salvo => "ui.desc.timing.salvo",
            TimingType::Delayed => "ui.desc.timing.delayed",
            TimingType::Alternating => "ui.desc.timing.alternating",
            TimingType::Charge => "ui.desc.timing.charge",
            TimingType::Gatling => "ui.desc.timing.gatling",
            TimingType::Heartbeat => "ui.desc.timing.heartbeat",
            TimingType::DoubleTap => "ui.desc.timing.double_tap",
        },
        ProgramParams::DamageMod { mod_type, .. } => match mod_type {
            DamageModType::Lifesteal => "ui.desc.damage_mod.lifesteal",
            DamageModType::ArmorPen => "ui.desc.damage_mod.armor_pen",
            DamageModType::BonusVsElite => "ui.desc.damage_mod.bonus_vs_elite",
            DamageModType::BonusVsBoss => "ui.desc.damage_mod.bonus_vs_boss",
            DamageModType::Execute => "ui.desc.damage_mod.execute",
            DamageModType::FlatDamage => "ui.desc.damage_mod.flat_damage",
            DamageModType::DamagePercent => "ui.desc.damage_mod.damage_percent",
            DamageModType::CritBoost => "ui.desc.damage_mod.crit_boost",
            DamageModType::CritDamage => "ui.desc.damage_mod.crit_damage",
            DamageModType::Overkill => "ui.desc.damage_mod.overkill",
            DamageModType::Ramp => "ui.desc.damage_mod.ramp",
            DamageModType::GlassCannon => "ui.desc.damage_mod.glass_cannon",
        },
        ProgramParams::ProjectileMod { mod_type, .. } => match mod_type {
            ProjectileModType::Chain => "ui.desc.projectile_mod.chain",
            ProjectileModType::Split => "ui.desc.projectile_mod.split",
            ProjectileModType::Explode => "ui.desc.projectile_mod.explode",
            ProjectileModType::Return => "ui.desc.projectile_mod.return",
            ProjectileModType::GrowSize => "ui.desc.projectile_mod.grow_size",
            ProjectileModType::ShrinkSize => "ui.desc.projectile_mod.shrink_size",
            ProjectileModType::Fragment => "ui.desc.projectile_mod.fragment",
            ProjectileModType::Bounce => "ui.desc.projectile_mod.bounce",
            ProjectileModType::Ghost => "ui.desc.projectile_mod.ghost",
            ProjectileModType::Attach => "ui.desc.projectile_mod.attach",
            ProjectileModType::Clone => "ui.desc.projectile_mod.clone",
            ProjectileModType::OrbitHit => "ui.desc.projectile_mod.orbit_hit",
        },
        ProgramParams::FieldMod { field_type, .. } => match field_type {
            FieldType::DamageZone => "ui.desc.field.damage_zone",
            FieldType::SlowField => "ui.desc.field.slow_field",
            FieldType::HealZone => "ui.desc.field.heal_zone",
            FieldType::GravityPull => "ui.desc.field.gravity_pull",
            FieldType::MineField => "ui.desc.field.mine_field",
            FieldType::Shield => "ui.desc.field.shield",
            FieldType::Beacon => "ui.desc.field.beacon",
            FieldType::Vortex => "ui.desc.field.vortex",
        },
    }
}

// ─── 工具函数 ───

/// Disc 品质颜色（egui Color32）
pub fn quality_color(q: DiscQuality) -> egui::Color32 {
    match q {
        DiscQuality::Common => EguiColors::QUALITY_COMMON,
        DiscQuality::Uncommon => EguiColors::QUALITY_UNCOMMON,
        DiscQuality::Rare => EguiColors::QUALITY_RARE,
        DiscQuality::Epic => EguiColors::QUALITY_EPIC,
        DiscQuality::Legendary => EguiColors::QUALITY_LEGENDARY,
    }
}

/// Program 类型颜色（egui Color32）
pub fn program_type_color(pt: ProgramType) -> egui::Color32 {
    match pt {
        ProgramType::Projectile => EguiColors::PROGRAM_PROJECTILE,
        ProgramType::TrajectoryMod => EguiColors::PROGRAM_TRAJECTORY,
        ProgramType::QuantityMod => EguiColors::PROGRAM_QUANTITY,
        ProgramType::Trigger => EguiColors::PROGRAM_TRIGGER,
        ProgramType::StatusEffect => EguiColors::PROGRAM_STATUS,
        ProgramType::FormationMod => EguiColors::PROGRAM_FORMATION,
        ProgramType::TimingMod => EguiColors::PROGRAM_TIMING,
        ProgramType::DamageMod => EguiColors::PROGRAM_DAMAGE_MOD,
        ProgramType::ProjectileMod => EguiColors::PROGRAM_PROJECTILE_MOD,
        ProgramType::FieldMod => EguiColors::PROGRAM_FIELD,
    }
}

/// Program 类型图标
pub fn program_type_icon(pt: ProgramType) -> &'static str {
    match pt {
        ProgramType::Projectile => "\u{25CF}",    // ●
        ProgramType::TrajectoryMod => "\u{2197}", // ↗
        ProgramType::QuantityMod => "\u{2726}",   // ✦
        ProgramType::Trigger => "\u{26A1}",       // ⚡
        ProgramType::StatusEffect => "\u{2727}",  // ✧
        ProgramType::FormationMod => "F",
        ProgramType::TimingMod => "T",
        ProgramType::DamageMod => "D",
        ProgramType::ProjectileMod => "M",
        ProgramType::FieldMod => "Z",
    }
}

/// 格式化物品 tooltip
pub fn format_item_tooltip(item: &ClientItem) -> String {
    match item {
        ClientItem::Disc(d) => {
            let mut s = t("ui.tooltip.disc_info")
                .replace("{0}", &t(quality_i18n_key(&d.quality)))
                .replace("{1}", &format!("{:.1}", 1.0 / d.base_fire_rate))
                .replace("{2}", &format!("{:.1}\u{00B0}", d.base_spread.to_degrees()))
                .replace("{3}", &d.slots.len().to_string());
            // 显示 Projectile 槽位中的额外属性
            for prog in d.slots.iter().flatten() {
                if let ProgramParams::Projectile {
                    pierce,
                    crit_chance,
                    knockback,
                    ..
                } = &prog.params
                {
                    if *pierce > 0 {
                        s.push_str(&format!("\n{}: {}", t("ui.tooltip.pierce"), pierce));
                    }
                    if *crit_chance > 0.0 {
                        s.push_str(&format!(
                            "\n{}: {:.0}%",
                            t("ui.tooltip.crit_chance"),
                            crit_chance * 100.0
                        ));
                    }
                    if *knockback > 0.0 {
                        s.push_str(&format!(
                            "\n{}: {:.1}",
                            t("ui.tooltip.knockback"),
                            knockback
                        ));
                    }
                }
            }
            s
        }
        ClientItem::Program(p) => format_program_tooltip(p),
    }
}

/// 格式化 Program tooltip（含 params 详细参数 + 功能描述）
pub fn format_program_tooltip(p: &ClientProgram) -> String {
    let type_str = t(program_type_i18n_key(p.program_type));
    let mut s = t("ui.tooltip.program_header")
        .replace("{0}", &type_str)
        .replace("{1}", &p.program_id.to_string());
    match &p.params {
        ProgramParams::Projectile {
            damage,
            speed,
            lifetime,
            size,
            ..
        } => {
            s.push('\n');
            s.push_str(
                &t("ui.tooltip.projectile_stats")
                    .replace("{0}", &format!("{:.0}", damage))
                    .replace("{1}", &format!("{:.0}", speed))
                    .replace("{2}", &format!("{:.1}", lifetime))
                    .replace("{3}", &format!("{:.0}", size)),
            );
        }
        ProgramParams::TrajectoryMod { mod_type } => {
            s.push('\n');
            s.push_str(
                &t("ui.tooltip.trajectory").replace("{0}", &t(trajectory_i18n_key(*mod_type))),
            );
        }
        ProgramParams::QuantityMod { count } => {
            s.push('\n');
            s.push_str(&t("ui.tooltip.multi").replace("{0}", &count.to_string()));
        }
        ProgramParams::Trigger {
            trigger_type,
            payload_slots,
        } => {
            s.push('\n');
            s.push_str(
                &t("ui.tooltip.trigger")
                    .replace("{0}", &t(trigger_i18n_key(*trigger_type)))
                    .replace("{1}", &payload_slots.to_string()),
            );
        }
        ProgramParams::StatusEffect {
            effect_type,
            duration,
            magnitude,
        } => {
            s.push('\n');
            s.push_str(
                &t("ui.tooltip.status_effect")
                    .replace("{0}", &t(status_i18n_key(*effect_type)))
                    .replace("{1}", &format!("{:.1}", duration))
                    .replace("{2}", &format!("{:.1}", magnitude)),
            );
        }
        ProgramParams::FormationMod { formation } => {
            s.push('\n');
            s.push_str(
                &t("ui.tooltip.formation").replace("{0}", &t(formation_i18n_key(*formation))),
            );
        }
        ProgramParams::TimingMod { timing } => {
            s.push('\n');
            s.push_str(&t("ui.tooltip.timing").replace("{0}", &t(timing_i18n_key(*timing))));
        }
        ProgramParams::DamageMod { mod_type, value } => {
            s.push('\n');
            s.push_str(
                &t("ui.tooltip.damage_mod")
                    .replace("{0}", &t(damage_mod_i18n_key(*mod_type)))
                    .replace("{1}", &format!("{:.0}%", value)),
            );
        }
        ProgramParams::ProjectileMod {
            mod_type,
            value,
            count,
        } => {
            s.push('\n');
            s.push_str(
                &t("ui.tooltip.projectile_mod")
                    .replace("{0}", &t(projectile_mod_i18n_key(*mod_type)))
                    .replace("{1}", &format!("{:.1}", value))
                    .replace("{2}", &count.to_string()),
            );
        }
        ProgramParams::FieldMod {
            field_type,
            radius,
            duration,
            magnitude,
        } => {
            s.push('\n');
            s.push_str(
                &t("ui.tooltip.field_mod")
                    .replace("{0}", &t(field_i18n_key(*field_type)))
                    .replace("{1}", &format!("{:.0}", radius))
                    .replace("{2}", &format!("{:.1}", duration))
                    .replace("{3}", &format!("{:.1}", magnitude)),
            );
        }
    }
    // 追加功能描述
    s.push('\n');
    s.push_str(&t(program_desc_key(&p.params)));
    s
}

use crate::data::*;

// ─── 2.1 新增 Modifier Instance 类型 ───

#[derive(Clone, Debug)]
pub struct DamageModInstance {
    pub mod_type: DamageModType,
    pub value: f32,
}

#[derive(Clone, Debug)]
pub struct ProjectileModInstance {
    pub mod_type: ProjectileModType,
    pub value: f32,
    pub count: u8,
}

#[derive(Clone, Debug)]
pub struct FieldModInstance {
    pub field_type: FieldType,
    pub radius: f32,
    pub duration: f32,
    pub magnitude: f32,
}

// ─── 执行引擎产出的弹射体（2.2 扩展） ───

/// 执行引擎产出的弹射体
#[derive(Clone, Debug)]
pub struct SpawnedProjectile {
    pub pos: (f32, f32),
    pub vel: (f32, f32),
    pub damage: f32,
    pub speed: f32,
    pub lifetime: f32,
    pub size: f32,
    pub trajectory_mods: Vec<TrajectoryType>,
    pub on_hit: Option<TriggerPayload>,
    pub on_kill: Option<TriggerPayload>,
    pub on_expire: Option<TriggerPayload>,
    pub applied_effects: Vec<StatusEffectInstance>,
    // Phase 2 新增字段
    pub pierce: u8,
    pub crit_chance: f32,
    pub crit_multiplier: f32,
    pub knockback: f32,
    pub formation: Option<FormationType>,
    pub timing: Option<TimingType>,
    pub damage_mods: Vec<DamageModInstance>,
    pub projectile_mods: Vec<ProjectileModInstance>,
    pub field_mods: Vec<FieldModInstance>,
    // 新增 Trigger slot 字段
    pub on_crit: Option<Vec<Program>>,
    pub on_boss_hit: Option<Vec<Program>>,
    pub on_periodic: Option<Vec<Program>>,
    pub on_low_hp: Option<Vec<Program>>,
    pub on_dodge: Option<Vec<Program>>,
    pub on_status_apply: Option<Vec<Program>>,
    pub on_multi_kill: Option<Vec<Program>>,
    pub on_full_hp: Option<Vec<Program>>,
}

#[derive(Clone, Debug)]
pub struct TriggerPayload {
    pub programs: Vec<Program>,
}

#[derive(Clone, Debug)]
pub struct StatusEffectInstance {
    pub effect_type: StatusEffectType,
    pub duration: f32,
    pub magnitude: f32,
}

// ─── Disc 执行器（2.3 扩展） ───

/// Disc 执行器：遍历 slot 序列，产出 SpawnedProjectile 列表
pub struct DiscExecutor<'a> {
    disc: &'a Disc,
    cursor: usize,
    pending_trajectory_mods: Vec<TrajectoryType>,
    pending_quantity: Option<u32>,
    pending_effects: Vec<StatusEffectInstance>,
    // Phase 2 新增 pending 字段
    pending_formation: Option<FormationType>,
    pending_timing: Option<TimingType>,
    pending_damage_mods: Vec<DamageModInstance>,
    pending_projectile_mods: Vec<ProjectileModInstance>,
    pending_field_mods: Vec<FieldModInstance>,
}

impl<'a> DiscExecutor<'a> {
    pub fn new(disc: &'a Disc) -> Self {
        Self {
            disc,
            cursor: 0,
            pending_trajectory_mods: Vec::new(),
            pending_quantity: None,
            pending_effects: Vec::new(),
            pending_formation: None,
            pending_timing: None,
            pending_damage_mods: Vec::new(),
            pending_projectile_mods: Vec::new(),
            pending_field_mods: Vec::new(),
        }
    }

    /// 从玩家位置和瞄准方向执行 Disc，返回所有生成的弹射体
    ///
    /// 严格顺序管道：从左到右逐 slot 执行
    /// - Modifier 累积到上下文中，遇到 Projectile 时用当前上下文生成弹射体
    /// - 靠前的 Modifier 影响更多 Projectile，slot 排列顺序决定 build 策略
    /// - Trigger 附加到紧邻前方的 Projectile，并消耗后续 payload slots
    pub fn execute(
        &mut self,
        player_pos: (f32, f32),
        aim_dir: (f32, f32),
    ) -> Vec<SpawnedProjectile> {
        let slots = &self.disc.slots;
        let mut result = Vec::new();
        self.cursor = 0;

        while self.cursor < slots.len() {
            let slot = &slots[self.cursor];
            self.cursor += 1;

            let program = match slot {
                Some(c) => c,
                None => continue,
            };

            match program.program_type {
                ProgramType::TrajectoryMod => {
                    if let ProgramParams::TrajectoryMod { mod_type } = &program.params {
                        self.pending_trajectory_mods.push(*mod_type);
                    }
                }
                ProgramType::QuantityMod => {
                    if let ProgramParams::QuantityMod { count } = &program.params {
                        self.pending_quantity = Some(self.pending_quantity.unwrap_or(1) * count);
                    }
                }
                ProgramType::StatusEffect => {
                    if let ProgramParams::StatusEffect {
                        effect_type,
                        duration,
                        magnitude,
                    } = &program.params
                    {
                        self.pending_effects.push(StatusEffectInstance {
                            effect_type: *effect_type,
                            duration: *duration,
                            magnitude: *magnitude,
                        });
                    }
                }
                // 2.4 新增 5 个 match 分支
                ProgramType::FormationMod => {
                    if let ProgramParams::FormationMod { formation } = &program.params {
                        self.pending_formation = Some(*formation);
                    }
                }
                ProgramType::TimingMod => {
                    if let ProgramParams::TimingMod { timing } = &program.params {
                        self.pending_timing = Some(*timing);
                    }
                }
                ProgramType::DamageMod => {
                    if let ProgramParams::DamageMod { mod_type, value } = &program.params {
                        self.pending_damage_mods.push(DamageModInstance {
                            mod_type: *mod_type,
                            value: *value,
                        });
                    }
                }
                ProgramType::ProjectileMod => {
                    if let ProgramParams::ProjectileMod {
                        mod_type,
                        value,
                        count,
                    } = &program.params
                    {
                        self.pending_projectile_mods.push(ProjectileModInstance {
                            mod_type: *mod_type,
                            value: *value,
                            count: *count,
                        });
                    }
                }
                ProgramType::FieldMod => {
                    if let ProgramParams::FieldMod {
                        field_type,
                        radius,
                        duration,
                        magnitude,
                    } = &program.params
                    {
                        self.pending_field_mods.push(FieldModInstance {
                            field_type: *field_type,
                            radius: *radius,
                            duration: *duration,
                            magnitude: *magnitude,
                        });
                    }
                }
                ProgramType::Projectile => {
                    let projs = self.make_projectiles(program, player_pos, aim_dir);
                    result.extend(projs);
                }
                ProgramType::Trigger => {
                    if let ProgramParams::Trigger {
                        trigger_type,
                        payload_slots,
                    } = &program.params
                    {
                        // 没有前置弹射体时跳过，不消耗 payload slots
                        if result.is_empty() {
                            continue;
                        }
                        // 收集后续 payload_slots 个非空 slot 作为 payload
                        let mut payload_programs = Vec::new();
                        let mut collected = 0u8;
                        while collected < *payload_slots && self.cursor < slots.len() {
                            let s = &slots[self.cursor];
                            self.cursor += 1;
                            if let Some(c) = s {
                                payload_programs.push(c.clone());
                                collected += 1;
                            }
                        }
                        // 附加到最后一个弹射体
                        let last = result.last_mut().unwrap();
                        // 2.5 扩展 Trigger 分支支持 8 种新 TriggerType
                        match trigger_type {
                            TriggerType::OnHit => {
                                last.on_hit = Some(TriggerPayload {
                                    programs: payload_programs,
                                });
                            }
                            TriggerType::OnKill => {
                                last.on_kill = Some(TriggerPayload {
                                    programs: payload_programs,
                                });
                            }
                            TriggerType::OnExpire => {
                                last.on_expire = Some(TriggerPayload {
                                    programs: payload_programs,
                                });
                            }
                            TriggerType::OnPierce => {
                                last.on_hit = Some(TriggerPayload {
                                    programs: payload_programs,
                                });
                            }
                            TriggerType::OnCrit => {
                                last.on_crit = Some(payload_programs);
                            }
                            TriggerType::OnBossHit => {
                                last.on_boss_hit = Some(payload_programs);
                            }
                            TriggerType::OnLowHp => {
                                last.on_low_hp = Some(payload_programs);
                            }
                            TriggerType::Periodic => {
                                last.on_periodic = Some(payload_programs);
                            }
                            TriggerType::OnDodge => {
                                last.on_dodge = Some(payload_programs);
                            }
                            TriggerType::OnStatusApply => {
                                last.on_status_apply = Some(payload_programs);
                            }
                            TriggerType::OnMultiKill => {
                                last.on_multi_kill = Some(payload_programs);
                            }
                            TriggerType::OnFullHp => {
                                last.on_full_hp = Some(payload_programs);
                            }
                        }
                    }
                }
            }
        }

        result
    }

    // 2.6 + 2.7 扩展 make_projectiles
    fn make_projectiles(
        &self,
        program: &Program,
        player_pos: (f32, f32),
        aim_dir: (f32, f32),
    ) -> Vec<SpawnedProjectile> {
        let (damage, speed, lifetime, size, pierce, crit_chance, crit_multiplier, knockback) =
            match &program.params {
                ProgramParams::Projectile {
                    damage,
                    speed,
                    lifetime,
                    size,
                    pierce,
                    crit_chance,
                    crit_multiplier,
                    knockback,
                } => (
                    *damage,
                    *speed,
                    *lifetime,
                    *size,
                    *pierce,
                    *crit_chance,
                    *crit_multiplier,
                    *knockback,
                ),
                _ => return Vec::new(),
            };

        let count = self.pending_quantity.unwrap_or(1);
        let base_angle = aim_dir.1.atan2(aim_dir.0);
        // 当 Multi 产生多个弹射体时，保证最小扇形角避免完全重叠
        let spread = if count > 1 {
            self.disc.base_spread.max(0.15)
        } else {
            self.disc.base_spread
        };

        let mut projs = Vec::with_capacity(count as usize);

        for i in 0..count {
            // 2.7 根据 pending_formation 决定角度分布
            let (angle, lateral_offset) =
                self.compute_projectile_angle(i, count, base_angle, spread);

            let vx = angle.cos() * speed;
            let vy = angle.sin() * speed;

            // 计算横向偏移后的位置（Parallel、Wall 等阵型用）
            let pos = if lateral_offset != 0.0 {
                // 垂直于瞄准方向的偏移
                let perp_x = -aim_dir.1;
                let perp_y = aim_dir.0;
                (
                    player_pos.0 + perp_x * lateral_offset,
                    player_pos.1 + perp_y * lateral_offset,
                )
            } else {
                player_pos
            };

            projs.push(SpawnedProjectile {
                pos,
                vel: (vx, vy),
                damage,
                speed,
                lifetime,
                size,
                trajectory_mods: self.pending_trajectory_mods.clone(),
                on_hit: None,
                on_kill: None,
                on_expire: None,
                applied_effects: self.pending_effects.clone(),
                // 2.6 新增字段
                pierce,
                crit_chance,
                crit_multiplier,
                knockback,
                formation: self.pending_formation,
                timing: self.pending_timing,
                damage_mods: self.pending_damage_mods.clone(),
                projectile_mods: self.pending_projectile_mods.clone(),
                field_mods: self.pending_field_mods.clone(),
                on_crit: None,
                on_boss_hit: None,
                on_periodic: None,
                on_low_hp: None,
                on_dodge: None,
                on_status_apply: None,
                on_multi_kill: None,
                on_full_hp: None,
            });
        }

        projs
    }

    /// 2.7 根据 FormationType 计算每个弹射体的角度和横向偏移
    /// 返回 (angle, lateral_offset)
    fn compute_projectile_angle(
        &self,
        index: u32,
        count: u32,
        base_angle: f32,
        spread: f32,
    ) -> (f32, f32) {
        match self.pending_formation {
            None | Some(FormationType::Fan) => {
                // 默认扇形：count 个弹射体均匀分布在 [-spread/2, spread/2]
                let angle_offset = if count > 1 {
                    let t = index as f32 / (count - 1) as f32;
                    spread * (t - 0.5)
                } else {
                    0.0
                };
                (base_angle + angle_offset, 0.0)
            }
            Some(FormationType::Parallel) => {
                // 所有弹射体同方向，横向偏移 ±spacing
                let spacing = 16.0;
                let offset = if count > 1 {
                    let t = index as f32 / (count - 1) as f32;
                    spacing * (count - 1) as f32 * (t - 0.5)
                } else {
                    0.0
                };
                (base_angle, offset)
            }
            Some(FormationType::Cross) => {
                // 十字形：0, 90, 180, 270 度
                let cross_angle = (index % 4) as f32 * std::f32::consts::FRAC_PI_2;
                (base_angle + cross_angle, 0.0)
            }
            Some(FormationType::Star) => {
                // 五角星：每 72 度
                let star_angle = (index % 5) as f32 * (2.0 * std::f32::consts::PI / 5.0);
                (base_angle + star_angle, 0.0)
            }
            Some(FormationType::Ring) => {
                // 360 度均分
                let ring_angle = if count > 0 {
                    index as f32 * (2.0 * std::f32::consts::PI / count as f32)
                } else {
                    0.0
                };
                (base_angle + ring_angle, 0.0)
            }
            Some(FormationType::Wall) => {
                // 垂直于瞄准方向的一排，同方向发射
                let spacing = 16.0;
                let offset = if count > 1 {
                    let t = index as f32 / (count - 1) as f32;
                    spacing * (count - 1) as f32 * (t - 0.5)
                } else {
                    0.0
                };
                (base_angle, offset)
            }
            Some(FormationType::Cone) => {
                // 窄扇形：spread / 2
                let narrow_spread = spread * 0.5;
                let angle_offset = if count > 1 {
                    let t = index as f32 / (count - 1) as f32;
                    narrow_spread * (t - 0.5)
                } else {
                    0.0
                };
                (base_angle + angle_offset, 0.0)
            }
            Some(FormationType::Random) => {
                // 确定性伪随机：基于 index 的哈希生成角度偏移
                let hash = ((index.wrapping_mul(2654435761)) >> 16) as f32 / 65535.0;
                let angle_offset = spread * (hash - 0.5);
                (base_angle + angle_offset, 0.0)
            }
            Some(FormationType::SpiralForm) => {
                // 每个弹射体递增固定角度（黄金角 ~137.5 度）
                let golden_angle = 2.399_963; // 约 137.508 度（弧度）
                let angle_offset = index as f32 * golden_angle;
                (base_angle + angle_offset, 0.0)
            }
            Some(FormationType::Backshot) => {
                // 一半向前一半向后
                if count <= 1 {
                    (base_angle, 0.0)
                } else {
                    let half = count / 2;
                    if index < half {
                        // 前半部分：向前扇形
                        let forward_spread = spread;
                        let t = if half > 1 {
                            index as f32 / (half - 1) as f32
                        } else {
                            0.5
                        };
                        let angle_offset = forward_spread * (t - 0.5);
                        (base_angle + angle_offset, 0.0)
                    } else {
                        // 后半部分：向后扇形
                        let back_index = index - half;
                        let back_count = count - half;
                        let backward_spread = spread;
                        let t = if back_count > 1 {
                            back_index as f32 / (back_count - 1) as f32
                        } else {
                            0.5
                        };
                        let angle_offset = backward_spread * (t - 0.5);
                        (base_angle + std::f32::consts::PI + angle_offset, 0.0)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2.8 更新 bullet_program 保持现有字段
    fn bullet_program(id: u32) -> Program {
        Program {
            program_id: id,
            program_type: ProgramType::Projectile,
            params: ProgramParams::Projectile {
                damage: 25.0,
                speed: 300.0,
                lifetime: 2.0,
                size: 4.0,
                pierce: 0,
                crit_chance: 0.0,
                crit_multiplier: 2.0,
                knockback: 0.0,
            },
        }
    }

    fn homing_program() -> Program {
        Program {
            program_id: 100,
            program_type: ProgramType::TrajectoryMod,
            params: ProgramParams::TrajectoryMod {
                mod_type: TrajectoryType::Homing,
            },
        }
    }

    fn scatter_program(count: u32) -> Program {
        Program {
            program_id: 200,
            program_type: ProgramType::QuantityMod,
            params: ProgramParams::QuantityMod { count },
        }
    }

    fn trigger_on_hit_program(payload_slots: u8) -> Program {
        Program {
            program_id: 300,
            program_type: ProgramType::Trigger,
            params: ProgramParams::Trigger {
                trigger_type: TriggerType::OnHit,
                payload_slots,
            },
        }
    }

    fn poison_program() -> Program {
        Program {
            program_id: 400,
            program_type: ProgramType::StatusEffect,
            params: ProgramParams::StatusEffect {
                effect_type: StatusEffectType::Poison,
                duration: 3.0,
                magnitude: 5.0,
            },
        }
    }

    fn explosion_program() -> Program {
        // 用 projectile 类型代表爆炸效果
        Program {
            program_id: 500,
            program_type: ProgramType::Projectile,
            params: ProgramParams::Projectile {
                damage: 50.0,
                speed: 0.0,
                lifetime: 0.5,
                size: 32.0,
                pierce: 0,
                crit_chance: 0.0,
                crit_multiplier: 2.0,
                knockback: 0.0,
            },
        }
    }

    fn make_disc(slots: Vec<Option<Program>>) -> Disc {
        Disc {
            quality: DiscQuality::Common,
            slots,
            base_fire_rate: 1.0,
            base_spread: 0.3, // ~17 度总扇形角
        }
    }

    #[test]
    fn single_bullet() {
        let disc = make_disc(vec![Some(bullet_program(1))]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].damage, 25.0);
        assert!(projs[0].trajectory_mods.is_empty());
        assert!(projs[0].applied_effects.is_empty());
        // 验证 Phase 2 新增字段默认值
        assert_eq!(projs[0].pierce, 0);
        assert_eq!(projs[0].crit_chance, 0.0);
        assert_eq!(projs[0].crit_multiplier, 2.0);
        assert_eq!(projs[0].knockback, 0.0);
        assert!(projs[0].formation.is_none());
        assert!(projs[0].timing.is_none());
        assert!(projs[0].damage_mods.is_empty());
        assert!(projs[0].projectile_mods.is_empty());
        assert!(projs[0].field_mods.is_empty());
    }

    #[test]
    fn homing_before_bullet() {
        // 严格顺序：homing 在 bullet 前，生效
        let disc = make_disc(vec![Some(homing_program()), Some(bullet_program(1))]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].trajectory_mods, vec![TrajectoryType::Homing]);
    }

    #[test]
    fn homing_after_projectile_no_effect() {
        // 严格顺序：Projectile 在前，Modifier 在后，不影响已生成的弹射体
        let disc = make_disc(vec![Some(bullet_program(1)), Some(homing_program())]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!(projs[0].trajectory_mods.is_empty());
    }

    #[test]
    fn scatter_three_bullets() {
        let disc = make_disc(vec![Some(scatter_program(3)), Some(bullet_program(1))]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 3);
        let vy_first = projs[0].vel.1;
        let vy_last = projs[2].vel.1;
        assert!(vy_first < 0.0, "第一个弹射体应偏向负 y（上方）");
        assert!(vy_last > 0.0, "最后一个弹射体应偏向正 y（下方）");
    }

    #[test]
    fn trigger_on_hit_with_payload() {
        // [子弹, 触发(命中时, 2), 爆炸, 中毒]
        let disc = make_disc(vec![
            Some(bullet_program(1)),
            Some(trigger_on_hit_program(2)),
            Some(explosion_program()),
            Some(poison_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        let p = &projs[0];
        assert!(p.on_hit.is_some());
        let payload = p.on_hit.as_ref().unwrap();
        assert_eq!(payload.programs.len(), 2);
        assert_eq!(payload.programs[0].program_id, 500); // 爆炸
        assert_eq!(payload.programs[1].program_id, 400); // 中毒
    }

    #[test]
    fn status_effect_on_bullet() {
        // [中毒, 子弹]
        let disc = make_disc(vec![Some(poison_program()), Some(bullet_program(1))]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].applied_effects.len(), 1);
        assert_eq!(
            projs[0].applied_effects[0].effect_type,
            StatusEffectType::Poison
        );
        assert_eq!(projs[0].applied_effects[0].duration, 3.0);
        assert_eq!(projs[0].applied_effects[0].magnitude, 5.0);
    }

    #[test]
    fn empty_disc_no_projectiles() {
        let disc = make_disc(vec![None, None, None]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert!(projs.is_empty());
    }

    #[test]
    fn modifier_without_projectile_produces_nothing() {
        let disc = make_disc(vec![Some(homing_program()), Some(scatter_program(3))]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert!(projs.is_empty());
    }

    #[test]
    fn multiple_projectiles_share_modifiers() {
        // 严格顺序：modifier 在前，对后续所有 projectile 生效
        let disc = make_disc(vec![
            Some(homing_program()),
            Some(bullet_program(1)),
            Some(bullet_program(2)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 2);
        assert_eq!(projs[0].trajectory_mods, vec![TrajectoryType::Homing]);
        assert_eq!(projs[1].trajectory_mods, vec![TrajectoryType::Homing]);
    }

    #[test]
    fn quantity_mod_applies_to_all_projectiles() {
        // scatter(3) + 两个 bullet = 每个 bullet 各生成 3 个 = 6 个
        let disc = make_disc(vec![
            Some(scatter_program(3)),
            Some(bullet_program(1)),
            Some(bullet_program(2)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 6); // 3 + 3
    }

    #[test]
    fn quantity_mod_stacks_multiplicatively() {
        // Multi(2) * Multi(2) = 4 个弹射体（Modifier 在 Projectile 前）
        let disc = make_disc(vec![
            Some(scatter_program(2)),
            Some(scatter_program(2)),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 4);
    }

    // ─── 严格顺序测试 ───

    #[test]
    fn status_after_projectile_no_effect() {
        // 严格顺序：StatusEffect 在 Projectile 后，不影响已生成弹射体
        let disc = make_disc(vec![Some(bullet_program(1)), Some(poison_program())]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!(projs[0].applied_effects.is_empty());
    }

    #[test]
    fn scatter_after_projectile_no_effect() {
        // 严格顺序：QuantityMod 在 Projectile 后，不影响已生成弹射体
        let disc = make_disc(vec![Some(bullet_program(1)), Some(scatter_program(3))]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
    }

    #[test]
    fn modifiers_only_affect_subsequent_projectiles() {
        // 严格顺序：Modifier 只影响其后的 Projectile
        // Multi(2) 在 Bullet 前 → 生成 2 个；Homing + Poison 在 Bullet 后 → 不生效
        let disc = make_disc(vec![
            Some(scatter_program(2)),
            Some(bullet_program(1)),
            Some(homing_program()),
            Some(poison_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 2);
        for p in &projs {
            assert!(p.trajectory_mods.is_empty());
            assert!(p.applied_effects.is_empty());
        }
    }

    #[test]
    fn modifier_between_projectiles_only_affects_later() {
        // [Bolt1, Homing, Bolt2] → Bolt1 无 Homing，Bolt2 有 Homing
        let disc = make_disc(vec![
            Some(bullet_program(1)),
            Some(homing_program()),
            Some(bullet_program(2)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 2);
        assert!(projs[0].trajectory_mods.is_empty());
        assert_eq!(projs[1].trajectory_mods, vec![TrajectoryType::Homing]);
    }

    // ─── 多 Modifier 叠加测试 ───

    #[test]
    fn multiple_trajectory_mods_stack() {
        // 两种轨迹修改都附加到弹射体
        let spiral = Program {
            program_id: 101,
            program_type: ProgramType::TrajectoryMod,
            params: ProgramParams::TrajectoryMod {
                mod_type: TrajectoryType::Spiral,
            },
        };
        let disc = make_disc(vec![
            Some(homing_program()),
            Some(spiral),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].trajectory_mods.len(), 2);
        assert!(projs[0].trajectory_mods.contains(&TrajectoryType::Homing));
        assert!(projs[0].trajectory_mods.contains(&TrajectoryType::Spiral));
    }

    #[test]
    fn multiple_status_effects_stack() {
        // 两种状态效果都附加到弹射体
        let burn = Program {
            program_id: 401,
            program_type: ProgramType::StatusEffect,
            params: ProgramParams::StatusEffect {
                effect_type: StatusEffectType::Burn,
                duration: 2.0,
                magnitude: 10.0,
            },
        };
        let disc = make_disc(vec![
            Some(poison_program()),
            Some(burn),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].applied_effects.len(), 2);
        let types: Vec<_> = projs[0]
            .applied_effects
            .iter()
            .map(|e| e.effect_type)
            .collect();
        assert!(types.contains(&StatusEffectType::Poison));
        assert!(types.contains(&StatusEffectType::Burn));
    }

    #[test]
    fn quantity_triple_stack() {
        // Multi(2) * Multi(3) = 6 个弹射体
        let disc = make_disc(vec![
            Some(scatter_program(2)),
            Some(scatter_program(3)),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 6);
    }

    // ─── Trigger 测试 ───

    #[test]
    fn trigger_without_preceding_projectile_skipped() {
        // Trigger 前面没有 Projectile，Trigger 被跳过，不消耗 payload
        // explosion_program 作为普通 Projectile 正常发射
        let disc = make_disc(vec![
            Some(trigger_on_hit_program(1)),
            Some(explosion_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!(projs[0].on_hit.is_none()); // Trigger 被跳过，无附加
    }

    #[test]
    fn trigger_on_kill() {
        let trigger_kill = Program {
            program_id: 301,
            program_type: ProgramType::Trigger,
            params: ProgramParams::Trigger {
                trigger_type: TriggerType::OnKill,
                payload_slots: 1,
            },
        };
        let disc = make_disc(vec![
            Some(bullet_program(1)),
            Some(trigger_kill),
            Some(explosion_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!(projs[0].on_kill.is_some());
        assert_eq!(projs[0].on_kill.as_ref().unwrap().programs.len(), 1);
    }

    #[test]
    fn trigger_on_expire() {
        let trigger_expire = Program {
            program_id: 302,
            program_type: ProgramType::Trigger,
            params: ProgramParams::Trigger {
                trigger_type: TriggerType::OnExpire,
                payload_slots: 1,
            },
        };
        let disc = make_disc(vec![
            Some(bullet_program(1)),
            Some(trigger_expire),
            Some(explosion_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!(projs[0].on_expire.is_some());
    }

    #[test]
    fn trigger_consumes_only_payload_slots_count() {
        // Trigger(payload_slots=1) 只消耗 1 个后续 slot，剩余 Projectile 正常生成
        let disc = make_disc(vec![
            Some(bullet_program(1)),
            Some(trigger_on_hit_program(1)),
            Some(explosion_program()), // 被 Trigger 消耗
            Some(bullet_program(2)),   // 独立弹射体
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 2); // bullet(1) + bullet(2)
        assert!(projs[0].on_hit.is_some());
        assert!(projs[1].on_hit.is_none());
    }

    // ─── 空槽位测试 ───

    #[test]
    fn empty_slots_ignored() {
        let disc = make_disc(vec![
            None,
            Some(scatter_program(2)),
            None,
            Some(bullet_program(1)),
            None,
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 2);
    }

    #[test]
    fn all_empty_slots() {
        let disc = make_disc(vec![None, None, None, None]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert!(projs.is_empty());
    }

    // ─── 复杂组合测试 ───

    #[test]
    fn full_build_scatter_homing_poison_trigger() {
        // 完整 Build：Multi(2) + Homing + Poison + Bolt + Trigger(OnHit,1) + 爆炸
        let disc = make_disc(vec![
            Some(scatter_program(2)),
            Some(homing_program()),
            Some(poison_program()),
            Some(bullet_program(1)),
            Some(trigger_on_hit_program(1)),
            Some(explosion_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        // 2 个弹射体（scatter），各带 Homing + Poison
        assert_eq!(projs.len(), 2);
        for p in &projs {
            assert_eq!(p.trajectory_mods, vec![TrajectoryType::Homing]);
            assert_eq!(p.applied_effects.len(), 1);
            assert_eq!(p.applied_effects[0].effect_type, StatusEffectType::Poison);
        }
        // Trigger 附加到最后一个弹射体（bullet 产出的最后一个）
        // trigger_on_hit 消耗 explosion_program 作为 payload
        assert!(projs.last().unwrap().on_hit.is_some());
    }

    #[test]
    fn two_projectiles_with_modifiers_and_trigger() {
        // 严格顺序：Multi(2) + Homing 在前，Bolt1 + Bolt2 在后
        // 每个 Bolt 各 2 个 = 4 个弹射体，全带 Homing
        // Trigger 附加到最后一个弹射体
        let disc = make_disc(vec![
            Some(scatter_program(2)),
            Some(homing_program()),
            Some(bullet_program(1)),
            Some(bullet_program(2)),
            Some(trigger_on_hit_program(1)),
            Some(explosion_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 4); // 2 + 2
        for p in &projs {
            assert_eq!(p.trajectory_mods, vec![TrajectoryType::Homing]);
        }
        // Trigger 附加到最后一个弹射体
        assert!(projs[3].on_hit.is_some());
    }

    // ─── 弹射体角度分布测试 ───

    #[test]
    fn single_projectile_no_spread() {
        // 单个弹射体无角度偏移，直接沿瞄准方向
        let disc = make_disc(vec![Some(bullet_program(1))]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!((projs[0].vel.1).abs() < 0.001, "单弹射体应无 y 分量偏移");
    }

    #[test]
    fn scatter_spread_symmetric() {
        // Multi(3) 的扇形分布应对称
        let disc = make_disc(vec![Some(scatter_program(3)), Some(bullet_program(1))]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 3);
        // 中间弹射体 y 分量接近 0
        assert!((projs[1].vel.1).abs() < 0.001, "中间弹射体应无偏移");
        // 首末对称
        assert!(
            (projs[0].vel.1 + projs[2].vel.1).abs() < 0.001,
            "首末弹射体应关于中轴对称"
        );
    }

    // ─── 2.9 新增 disc_engine 测试 ───

    fn formation_mod_program(formation: FormationType) -> Program {
        Program {
            program_id: 600,
            program_type: ProgramType::FormationMod,
            params: ProgramParams::FormationMod { formation },
        }
    }

    fn damage_mod_program(mod_type: DamageModType, value: f32) -> Program {
        Program {
            program_id: 700,
            program_type: ProgramType::DamageMod,
            params: ProgramParams::DamageMod { mod_type, value },
        }
    }

    fn projectile_mod_program(mod_type: ProjectileModType, value: f32, count: u8) -> Program {
        Program {
            program_id: 800,
            program_type: ProgramType::ProjectileMod,
            params: ProgramParams::ProjectileMod {
                mod_type,
                value,
                count,
            },
        }
    }

    fn field_mod_program(
        field_type: FieldType,
        radius: f32,
        duration: f32,
        magnitude: f32,
    ) -> Program {
        Program {
            program_id: 900,
            program_type: ProgramType::FieldMod,
            params: ProgramParams::FieldMod {
                field_type,
                radius,
                duration,
                magnitude,
            },
        }
    }

    #[test]
    fn formation_mod_before_projectile_affects_formation() {
        // FormationMod 在 Projectile 前 → 影响 Projectile 的 formation 字段
        let disc = make_disc(vec![
            Some(formation_mod_program(FormationType::Ring)),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].formation, Some(FormationType::Ring));
    }

    #[test]
    fn damage_mod_before_projectile_transferred() {
        // DamageMod 在 Projectile 前 → 传递到 SpawnedProjectile.damage_mods
        let disc = make_disc(vec![
            Some(damage_mod_program(DamageModType::Lifesteal, 0.1)),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].damage_mods.len(), 1);
        assert_eq!(projs[0].damage_mods[0].mod_type, DamageModType::Lifesteal);
        assert_eq!(projs[0].damage_mods[0].value, 0.1);
    }

    #[test]
    fn projectile_mod_before_projectile_transferred() {
        // ProjectileMod 在 Projectile 前 → 传递到 SpawnedProjectile.projectile_mods
        let disc = make_disc(vec![
            Some(projectile_mod_program(ProjectileModType::Chain, 3.0, 2)),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].projectile_mods.len(), 1);
        assert_eq!(
            projs[0].projectile_mods[0].mod_type,
            ProjectileModType::Chain
        );
        assert_eq!(projs[0].projectile_mods[0].value, 3.0);
        assert_eq!(projs[0].projectile_mods[0].count, 2);
    }

    #[test]
    fn field_mod_before_projectile_transferred() {
        // FieldMod 在 Projectile 前 → 传递到 SpawnedProjectile.field_mods
        let disc = make_disc(vec![
            Some(field_mod_program(FieldType::DamageZone, 64.0, 3.0, 10.0)),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert_eq!(projs[0].field_mods.len(), 1);
        assert_eq!(projs[0].field_mods[0].field_type, FieldType::DamageZone);
        assert_eq!(projs[0].field_mods[0].radius, 64.0);
        assert_eq!(projs[0].field_mods[0].duration, 3.0);
        assert_eq!(projs[0].field_mods[0].magnitude, 10.0);
    }

    #[test]
    fn modifier_after_projectile_no_effect_phase2() {
        // 严格顺序管道：Modifier 在 Projectile 后放置 → 不影响该 Projectile
        let disc = make_disc(vec![
            Some(bullet_program(1)),
            Some(formation_mod_program(FormationType::Ring)),
            Some(damage_mod_program(DamageModType::Lifesteal, 0.1)),
            Some(projectile_mod_program(ProjectileModType::Chain, 3.0, 2)),
            Some(field_mod_program(FieldType::DamageZone, 64.0, 3.0, 10.0)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!(projs[0].formation.is_none());
        assert!(projs[0].damage_mods.is_empty());
        assert!(projs[0].projectile_mods.is_empty());
        assert!(projs[0].field_mods.is_empty());
    }

    #[test]
    fn new_trigger_types_on_crit() {
        // 2.5 验证新 TriggerType 的执行
        let trigger_on_crit = Program {
            program_id: 310,
            program_type: ProgramType::Trigger,
            params: ProgramParams::Trigger {
                trigger_type: TriggerType::OnCrit,
                payload_slots: 1,
            },
        };
        let disc = make_disc(vec![
            Some(bullet_program(1)),
            Some(trigger_on_crit),
            Some(explosion_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!(projs[0].on_crit.is_some());
        assert_eq!(projs[0].on_crit.as_ref().unwrap().len(), 1);
        assert_eq!(projs[0].on_crit.as_ref().unwrap()[0].program_id, 500);
    }

    #[test]
    fn new_trigger_types_on_boss_hit() {
        let trigger = Program {
            program_id: 311,
            program_type: ProgramType::Trigger,
            params: ProgramParams::Trigger {
                trigger_type: TriggerType::OnBossHit,
                payload_slots: 1,
            },
        };
        let disc = make_disc(vec![
            Some(bullet_program(1)),
            Some(trigger),
            Some(explosion_program()),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 1);
        assert!(projs[0].on_boss_hit.is_some());
        assert_eq!(projs[0].on_boss_hit.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn formation_ring_with_multi() {
        // Ring 阵型 + Multi(4)：4 个弹射体 360 度均分
        let disc = make_disc(vec![
            Some(formation_mod_program(FormationType::Ring)),
            Some(scatter_program(4)),
            Some(bullet_program(1)),
        ]);
        let mut exe = DiscExecutor::new(&disc);
        let projs = exe.execute((0.0, 0.0), (1.0, 0.0));

        assert_eq!(projs.len(), 4);
        // 每个弹射体都标记了 Ring 阵型
        for p in &projs {
            assert_eq!(p.formation, Some(FormationType::Ring));
        }
        // 验证角度大致均分（第 0 个和第 2 个应近似反向）
        let v0 = projs[0].vel;
        let v2 = projs[2].vel;
        // 反向意味着 vx 和 vy 近似相反
        assert!((v0.0 + v2.0).abs() < 1.0, "Ring 0 和 2 应近似反向");
        assert!((v0.1 + v2.1).abs() < 1.0, "Ring 0 和 2 应近似反向");
    }
}

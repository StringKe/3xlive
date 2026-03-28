//! 里程碑定义与检查逻辑

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::data::disc::DiscQuality;
use crate::data::program::ProgramType;
use crate::events::RunResult;

use super::state::MetaState;

/// 里程碑定义
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct MilestoneDefinition {
    pub id: u32,
    pub condition: MilestoneCondition,
    pub reward: UnlockReward,
}

/// 里程碑触发条件
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub enum MilestoneCondition {
    TotalKills(u32),
    TotalDamage(f32),
    SingleRunSurvival(f32),
    BossKills(u32),
}

/// 解锁奖励
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub enum UnlockReward {
    NewProgramType(ProgramType),
    ProgramLevel(ProgramType, u32),
    DiscQualityOption(DiscQuality),
}

/// 检查里程碑解锁
///
/// 返回本次新解锁的奖励列表，同时更新 MetaState 中的进度和解锁状态
pub fn check_milestones(
    meta: &mut MetaState,
    result: &RunResult,
    milestones: &[MilestoneDefinition],
) -> Vec<UnlockReward> {
    let mut new_rewards = Vec::new();

    for milestone in milestones {
        // 已解锁的里程碑跳过
        if meta.completed_milestones.contains(&milestone.id) {
            continue;
        }

        // 更新累计进度
        let progress = meta.milestone_progress.entry(milestone.id).or_insert(0.0);

        match &milestone.condition {
            MilestoneCondition::TotalKills(target) => {
                *progress += (result.kills_normal + result.kills_elite + result.kills_boss) as f32;
                if *progress >= *target as f32 {
                    meta.completed_milestones.insert(milestone.id);
                    apply_reward(meta, &milestone.reward);
                    new_rewards.push(milestone.reward.clone());
                }
            }
            MilestoneCondition::TotalDamage(target) => {
                *progress += result.total_damage;
                if *progress >= *target {
                    meta.completed_milestones.insert(milestone.id);
                    apply_reward(meta, &milestone.reward);
                    new_rewards.push(milestone.reward.clone());
                }
            }
            MilestoneCondition::SingleRunSurvival(target) => {
                // 单局最高存活时间
                if result.survived_secs > *progress {
                    *progress = result.survived_secs;
                }
                if *progress >= *target {
                    meta.completed_milestones.insert(milestone.id);
                    apply_reward(meta, &milestone.reward);
                    new_rewards.push(milestone.reward.clone());
                }
            }
            MilestoneCondition::BossKills(target) => {
                *progress += result.kills_boss as f32;
                if *progress >= *target as f32 {
                    meta.completed_milestones.insert(milestone.id);
                    apply_reward(meta, &milestone.reward);
                    new_rewards.push(milestone.reward.clone());
                }
            }
        }
    }

    new_rewards
}

fn apply_reward(meta: &mut MetaState, reward: &UnlockReward) {
    match reward {
        UnlockReward::NewProgramType(pt) => {
            meta.unlocked_program_types.insert(*pt);
        }
        UnlockReward::ProgramLevel(pt, level) => {
            let entry = meta.unlocked_program_levels.entry(*pt).or_insert(0);
            if *level > *entry {
                *entry = *level;
            }
        }
        UnlockReward::DiscQualityOption(quality) => {
            meta.unlocked_disc_qualities.insert(*quality);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_result(kills: u32, boss_kills: u32, survived: f32) -> RunResult {
        RunResult {
            survived_secs: survived,
            total_damage: kills as f32 * 10.0,
            kills_normal: kills,
            kills_elite: 0,
            kills_boss: boss_kills,
            max_chunk_difficulty: 1.0,
            final_level: 5,
        }
    }

    fn make_test_milestones() -> Vec<MilestoneDefinition> {
        vec![
            MilestoneDefinition {
                id: 1,
                condition: MilestoneCondition::TotalKills(100),
                reward: UnlockReward::NewProgramType(ProgramType::Trigger),
            },
            MilestoneDefinition {
                id: 2,
                condition: MilestoneCondition::BossKills(3),
                reward: UnlockReward::DiscQualityOption(DiscQuality::Uncommon),
            },
            MilestoneDefinition {
                id: 3,
                condition: MilestoneCondition::SingleRunSurvival(120.0),
                reward: UnlockReward::ProgramLevel(ProgramType::Projectile, 2),
            },
        ]
    }

    #[test]
    fn milestone_triggers_on_threshold() {
        let mut meta = MetaState::default();
        let milestones = make_test_milestones();
        let result = make_test_result(150, 0, 60.0);

        let rewards = check_milestones(&mut meta, &result, &milestones);

        assert_eq!(rewards.len(), 1);
        assert!(matches!(
            rewards[0],
            UnlockReward::NewProgramType(ProgramType::Trigger)
        ));
        assert!(meta.unlocked_program_types.contains(&ProgramType::Trigger));
        assert!(meta.completed_milestones.contains(&1));
    }

    #[test]
    fn milestone_does_not_double_trigger() {
        let mut meta = MetaState::default();
        let milestones = make_test_milestones();
        let result = make_test_result(150, 0, 60.0);

        check_milestones(&mut meta, &result, &milestones);
        let rewards2 = check_milestones(&mut meta, &result, &milestones);

        // 第二次不应再触发 milestone 1
        assert!(!rewards2
            .iter()
            .any(|r| matches!(r, UnlockReward::NewProgramType(ProgramType::Trigger))));
    }

    #[test]
    fn boss_kills_accumulate() {
        let mut meta = MetaState::default();
        let milestones = make_test_milestones();

        // 第一次 2 boss kills，未达 3
        let result1 = make_test_result(0, 2, 60.0);
        let rewards1 = check_milestones(&mut meta, &result1, &milestones);
        assert!(!rewards1
            .iter()
            .any(|r| matches!(r, UnlockReward::DiscQualityOption(DiscQuality::Uncommon))));

        // 第二次 2 boss kills，累计 4 >= 3
        let result2 = make_test_result(0, 2, 60.0);
        let rewards2 = check_milestones(&mut meta, &result2, &milestones);
        assert!(rewards2
            .iter()
            .any(|r| matches!(r, UnlockReward::DiscQualityOption(DiscQuality::Uncommon))));
        assert!(meta
            .unlocked_disc_qualities
            .contains(&DiscQuality::Uncommon));
    }

    #[test]
    fn single_run_survival_uses_max() {
        let mut meta = MetaState::default();
        let milestones = make_test_milestones();

        // 存活 60s，未达 120s
        let r1 = make_test_result(0, 0, 60.0);
        check_milestones(&mut meta, &r1, &milestones);
        assert!(!meta.completed_milestones.contains(&3));

        // 存活 130s >= 120s
        let r2 = make_test_result(0, 0, 130.0);
        let rewards = check_milestones(&mut meta, &r2, &milestones);
        assert!(rewards
            .iter()
            .any(|r| matches!(r, UnlockReward::ProgramLevel(ProgramType::Projectile, 2))));
    }
}

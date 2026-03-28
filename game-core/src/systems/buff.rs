//! Buff 系统：buff 效果应用 + buff 生命周期管理
//!
//! 从 game-server/src/buff.rs 迁入，使用 DeltaTime Resource 替代 TICK_DT 常量。

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;

use crate::components::*;
use crate::systems::DeltaTime;

/// Buff tick 系统：递减 remaining，到期移除
///
/// ExtraSlot buff 到期时缩减背包容量，将最后一格物品弹出为掉落物
/// （通过 crate::loot::spawn_loot_entity 生成）
pub fn buff_tick_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    let mut extra_slot_expired: Vec<Entity> = Vec::new();

    for (entity, mut buffs) in world.query::<(Entity, &mut ActiveBuffs)>().iter_mut(world) {
        let mut i = 0;
        while i < buffs.buffs.len() {
            buffs.buffs[i].remaining -= dt;
            if buffs.buffs[i].remaining <= 0.0 {
                let expired = buffs.buffs.swap_remove(i);
                if expired.buff_type == BuffType::ExtraSlot {
                    extra_slot_expired.push(entity);
                }
            } else {
                i += 1;
            }
        }
    }

    // ExtraSlot 到期：缩减背包容量（将最后一格物品弹出为掉落物）
    for entity in extra_slot_expired {
        let popped_item = {
            if let Some(mut inv) = world.get_mut::<Inventory>(entity) {
                if inv.max_slots > 2 {
                    let last_idx = inv.max_slots - 1;
                    let popped = if last_idx < inv.slots.len() {
                        inv.slots[last_idx].take()
                    } else {
                        None
                    };
                    inv.max_slots -= 1;
                    let new_len = inv.max_slots;
                    inv.slots.truncate(new_len);
                    popped
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(item) = popped_item {
            let pos = world
                .get::<Position>(entity)
                .map(|p| (p.x, p.y))
                .unwrap_or((0.0, 0.0));
            let loot_item = match item {
                InventoryItem::Disc(d) => LootItem::DiscDrop(d),
                InventoryItem::Program(p) => LootItem::ProgramDrop(p),
            };
            crate::loot::spawn_loot_entity(world, pos, loot_item);
        }
    }
}

/// Buff 效果应用系统：修改 Health.max（MaxHealth buff）
pub fn buff_apply_system(world: &mut World) {
    for (mut health, buffs) in world.query::<(&mut Health, &ActiveBuffs)>().iter_mut(world) {
        let bonus = buffs.get_magnitude(BuffType::MaxHealth);
        let base_max = 100.0_f32;
        health.max = base_max + bonus;
        if health.current > health.max {
            health.current = health.max;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEventQueue;

    #[test]
    fn buff_tick_decrements_remaining() {
        let mut world = World::default();
        let dt = crate::data::constants::TICK_DT;
        world.insert_resource(DeltaTime(dt));
        world.insert_resource(GameEventQueue::default());

        let entity = world
            .spawn(ActiveBuffs {
                buffs: vec![BuffInstance {
                    buff_type: BuffType::MoveSpeed,
                    magnitude: 0.3,
                    remaining: 1.0,
                }],
            })
            .id();

        buff_tick_system(&mut world);

        let buffs = world.get::<ActiveBuffs>(entity).unwrap();
        assert_eq!(buffs.buffs.len(), 1);
        assert!((buffs.buffs[0].remaining - (1.0 - dt)).abs() < 0.001);
    }

    #[test]
    fn buff_tick_removes_expired() {
        let mut world = World::default();
        let dt = crate::data::constants::TICK_DT;
        world.insert_resource(DeltaTime(dt));
        world.insert_resource(GameEventQueue::default());

        let entity = world
            .spawn(ActiveBuffs {
                buffs: vec![BuffInstance {
                    buff_type: BuffType::MoveSpeed,
                    magnitude: 0.3,
                    remaining: dt * 0.5, // 低于 dt，会到期
                }],
            })
            .id();

        buff_tick_system(&mut world);

        let buffs = world.get::<ActiveBuffs>(entity).unwrap();
        assert_eq!(buffs.buffs.len(), 0);
    }

    #[test]
    fn extra_slot_expired_shrinks_inventory() {
        let mut world = World::default();
        world.insert_resource(DeltaTime(1.0));
        world.insert_resource(GameEventQueue::default());

        let mut inv = Inventory::new(4);
        inv.slots[3] = Some(InventoryItem::Program(crate::data::Program {
            program_id: 1,
            program_type: crate::data::ProgramType::Projectile,
            params: crate::data::ProgramParams::Projectile {
                damage: 10.0,
                speed: 300.0,
                lifetime: 2.0,
                size: 6.0,
                pierce: 0,
                crit_chance: 0.0,
                crit_multiplier: 2.0,
                knockback: 0.0,
            },
        }));

        let entity = world
            .spawn((
                Position { x: 50.0, y: 50.0 },
                inv,
                ActiveBuffs {
                    buffs: vec![BuffInstance {
                        buff_type: BuffType::ExtraSlot,
                        magnitude: 1.0,
                        remaining: 0.5, // 会到期
                    }],
                },
            ))
            .id();

        buff_tick_system(&mut world);

        // 背包缩至 3 格
        let inv = world.get::<Inventory>(entity).unwrap();
        assert_eq!(inv.max_slots, 3);
        assert_eq!(inv.slots.len(), 3);

        // 弹出的物品变成掉落物
        let loot_count = world.query::<&LootDrop>().iter(&world).count();
        assert_eq!(loot_count, 1);
    }

    #[test]
    fn buff_apply_max_health() {
        let mut world = World::default();

        let entity = world
            .spawn((
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                ActiveBuffs {
                    buffs: vec![BuffInstance {
                        buff_type: BuffType::MaxHealth,
                        magnitude: 50.0,
                        remaining: 10.0,
                    }],
                },
            ))
            .id();

        buff_apply_system(&mut world);

        let health = world.get::<Health>(entity).unwrap();
        assert!((health.max - 150.0).abs() < 0.001);
    }

    #[test]
    fn buff_apply_clamps_current_hp() {
        let mut world = World::default();

        let entity = world
            .spawn((
                Health {
                    current: 120.0,
                    max: 150.0,
                },
                ActiveBuffs {
                    buffs: vec![], // 无 MaxHealth buff
                },
            ))
            .id();

        buff_apply_system(&mut world);

        let health = world.get::<Health>(entity).unwrap();
        // base_max = 100, 无 buff, current 120 > 100 → clamp
        assert!((health.max - 100.0).abs() < 0.001);
        assert!((health.current - 100.0).abs() < 0.001);
    }
}

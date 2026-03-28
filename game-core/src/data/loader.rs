use super::config::GameConfig;

/// 从 RON 字符串解析 GameConfig
pub fn load_config_from_ron(ron_str: &str) -> Result<GameConfig, String> {
    ron::from_str(ron_str).map_err(|e| format!("RON 解析失败: {e}"))
}

/// 将 GameConfig 序列化为 bincode bytes（用于 S2C::ConfigSync）
pub fn encode_config(config: &GameConfig) -> Vec<u8> {
    bincode::encode_to_vec(config, bincode::config::standard()).expect("bincode encode GameConfig")
}

/// 从 bincode bytes 反序列化 GameConfig（客户端接收 ConfigSync 后调用）
pub fn decode_config(bytes: &[u8]) -> Result<GameConfig, String> {
    let (config, _): (GameConfig, _) =
        bincode::decode_from_slice(bytes, bincode::config::standard())
            .map_err(|e| format!("bincode decode GameConfig 失败: {e}"))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ron_roundtrip() {
        let ron_str = r#"
GameConfig(
    discs: DiscTable(templates: [
        DiscTemplate(id: "basic_disc", quality: "Common", slots: 4, base_fire_rate: 1.0, base_spread: 0.1),
    ]),
    programs: ProgramTable(templates: [
        ProgramTemplate(id: "spark_bolt", program_type: "Projectile", damage: Some(8.0), speed: Some(400.0), lifetime: Some(2.0), size: Some(4.0), mod_type: None, count: None, trigger_type: None, payload_slots: None, effect_type: None, duration: None, magnitude: None),
    ]),
    monsters: MonsterTable(templates: [
        MonsterTemplate(id: "slime", hp: 20.0, speed: 60.0, damage: 5.0, xp_reward: 10, detection_radius: 256.0, traits: []),
    ]),
    loot: LootTable(entries: [
        LootEntry(monster_id: "slime", drops: [
            LootDropEntry(item_id: "xp_gem", weight: 1.0, min_count: 1, max_count: 3),
        ]),
    ]),
    waves: WaveConfig(waves: [
        WaveEntry(time_start: 0.0, time_end: 60.0, monster_ids: ["slime"], spawn_rate: 2.0, max_count: 20),
    ]),
)
"#;
        let config = load_config_from_ron(ron_str).unwrap();
        assert_eq!(config.discs.templates.len(), 1);
        assert_eq!(config.discs.templates[0].id, "basic_disc");

        // bincode 往返序列化
        let bytes = encode_config(&config);
        let decoded = decode_config(&bytes).unwrap();
        assert_eq!(decoded.monsters.templates[0].id, "slime");
    }
}

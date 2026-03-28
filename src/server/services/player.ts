const FP_RE = /^[a-f0-9]{16,40}$/;

export async function enterPlayer(
  db: D1Database,
  fingerprint: string,
  clerkUserId?: string,
  clerkDisplayName?: string,
): Promise<{ id: string; displayName: string; isNew: boolean }> {
  if (!FP_RE.test(fingerprint)) {
    throw new Error("Invalid fingerprint");
  }

  if (clerkUserId) {
    // 分支 1: Clerk 用户回访
    let row = await db
      .prepare("SELECT * FROM players WHERE clerk_user_id=?")
      .bind(clerkUserId)
      .first();
    if (row) {
      await db
        .prepare("UPDATE players SET fingerprint=?, last_seen_at=datetime('now') WHERE id=?")
        .bind(fingerprint, row.id)
        .run();
      return { id: row.id as string, displayName: row.display_name as string, isNew: false };
    }

    // 分支 2: 访客绑定 Clerk（原子性迁移）
    row = await db
      .prepare("SELECT * FROM players WHERE id=? AND clerk_user_id IS NULL")
      .bind(fingerprint)
      .first();
    if (row) {
      await db.batch([
        db
          .prepare(
            "INSERT INTO players (id,fingerprint,clerk_user_id,display_name,meta_unlocks,trust_score,created_at) VALUES(?,?,?,?,?,?,?)",
          )
          .bind(
            clerkUserId,
            fingerprint,
            clerkUserId,
            clerkDisplayName ?? row.display_name,
            (row.meta_unlocks as string) ?? "{}",
            (row.trust_score as number) ?? 50,
            row.created_at as string,
          ),
        db.prepare("DELETE FROM players WHERE id=?").bind(fingerprint),
      ]);
      return {
        id: clerkUserId,
        displayName: (clerkDisplayName ?? row.display_name) as string,
        isNew: false,
      };
    }

    // 分支 3: Clerk 用户首次
    await db
      .prepare("INSERT INTO players (id,fingerprint,clerk_user_id,display_name) VALUES(?,?,?,?)")
      .bind(clerkUserId, fingerprint, clerkUserId, clerkDisplayName ?? "Player")
      .run();
    return { id: clerkUserId, displayName: clerkDisplayName ?? "Player", isNew: true };
  }

  // 分支 4: 访客回访
  const row = await db.prepare("SELECT * FROM players WHERE id=?").bind(fingerprint).first();
  if (row) {
    await db
      .prepare("UPDATE players SET last_seen_at=datetime('now') WHERE id=?")
      .bind(fingerprint)
      .run();
    return { id: row.id as string, displayName: row.display_name as string, isNew: false };
  }

  // 分支 5: 访客首次
  await db
    .prepare("INSERT INTO players (id,fingerprint) VALUES(?,?)")
    .bind(fingerprint, fingerprint)
    .run();
  return { id: fingerprint, displayName: "Player", isNew: true };
}

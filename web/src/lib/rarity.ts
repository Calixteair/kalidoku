/**
 * Rarity tiers derived from an entity's `fame_score` (0..=100).
 *
 * 0 = very niche, 100 = household-name. We invert that into a 4-tier rarity
 * the player sees right after answering a cell. Thresholds picked with the
 * user (80/50/20) so the famous Châtelet-class stations stay 'Commun',
 * mid-known ones become 'Rare', and only deep cuts surface 'Légendaire'.
 *
 * Null / undefined fame is treated as 'Commun' on purpose — we don't want
 * to silently mislabel an un-scored station as a rare pick. A future
 * ingest pass can fill those entries; the UI doesn't need to know.
 */
export type Rarity = "common" | "rare" | "epic" | "legendary";

export const rarityFor = (fame: number | null | undefined): Rarity => {
  if (fame === null || fame === undefined) return "common";
  if (fame >= 80) return "common";
  if (fame >= 50) return "rare";
  if (fame >= 20) return "epic";
  return "legendary";
};

export const RARITY_ORDER: Rarity[] = ["common", "rare", "epic", "legendary"];

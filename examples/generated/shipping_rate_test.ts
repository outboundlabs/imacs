// GENERATED TESTS FROM: shipping_rate.yaml
// SPEC HASH: sha256:bfd80b5a15c6208e
// GENERATED: 2026-01-05T17:28:47.187124451+00:00
// DO NOT EDIT — regenerate from spec

import { describe, it, expect } from 'vitest';
import { shippingRate } from './shipping_rate';

describe('shippingRate', () => {
  describe('rules', () => {
    it('R1: member_tier == \'gold\' && zone == \'domestic\' → 0', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "domestic", priority: false, memberTier: "gold" })).toBe(0);
    });

    it('R2: priority && zone == \'international\' → "weight_kg * 25.0 + 50.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "international", priority: true, memberTier: "" })).toBe("weight_kg * 25.0 + 50.0");
    });

    it('R3: priority && zone == \'north_america\' → "weight_kg * 15.0 + 20.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "north_america", priority: true, memberTier: "" })).toBe("weight_kg * 15.0 + 20.0");
    });

    it('R4: priority && zone == \'domestic\' → "weight_kg * 8.0 + 10.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "domestic", priority: true, memberTier: "" })).toBe("weight_kg * 8.0 + 10.0");
    });

    it('R5: member_tier == \'silver\' && zone == \'international\' → "weight_kg * 16.0 + 30.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "international", priority: false, memberTier: "silver" })).toBe("weight_kg * 16.0 + 30.0");
    });

    it('R6: member_tier == \'silver\' && zone == \'north_america\' → "weight_kg * 8.0 + 12.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "north_america", priority: false, memberTier: "silver" })).toBe("weight_kg * 8.0 + 12.0");
    });

    it('R7: member_tier == \'silver\' && zone == \'domestic\' → "weight_kg * 4.0 + 5.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "domestic", priority: false, memberTier: "silver" })).toBe("weight_kg * 4.0 + 5.0");
    });

    it('R8: zone == \'international\' → "weight_kg * 20.0 + 40.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "international", priority: false, memberTier: "" })).toBe("weight_kg * 20.0 + 40.0");
    });

    it('R9: zone == \'north_america\' → "weight_kg * 10.0 + 15.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "north_america", priority: false, memberTier: "" })).toBe("weight_kg * 10.0 + 15.0");
    });

    it('R10: zone == \'domestic\' → "weight_kg * 5.0 + 7.0"', () => {
      expect(shippingRate({ weightKg: 0.0, zone: "domestic", priority: false, memberTier: "" })).toBe("weight_kg * 5.0 + 7.0");
    });

  });

});

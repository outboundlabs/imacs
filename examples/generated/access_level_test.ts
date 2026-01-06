// GENERATED TESTS FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.610768861+00:00
// DO NOT EDIT — regenerate from spec

import { describe, it, expect } from 'vitest';
import { accessLevel } from './access_level';

describe('accessLevel', () => {
  describe('rules', () => {
    it('R1: role == \'admin\' → 100', () => {
      expect(accessLevel({ role: "admin", verified: false })).toBe(100);
    });

    it('R2: role == \'member\' && verified → 50', () => {
      expect(accessLevel({ role: "member", verified: true })).toBe(50);
    });

    it('R3: role == \'member\' && !verified → 25', () => {
      expect(accessLevel({ role: "member", verified: false })).toBe(25);
    });

    it('R4: role == \'guest\' → 10', () => {
      expect(accessLevel({ role: "guest", verified: false })).toBe(10);
    });

  });

});

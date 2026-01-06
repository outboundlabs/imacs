// GENERATED TESTS FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.852451323+00:00
// DO NOT EDIT — regenerate from spec

import { describe, it, expect } from 'vitest';
import { orderFlow, OrderFlowError } from './order_flow';

describe('orderFlow', () => {
  it('should succeed with valid inputs', async () => {
    const input = {
      role: 'test',
      verified: true,
      weightKg: 10.0,
      zone: 'test',
      priority: true,
      memberTier: 'test',
    };

    await expect(orderFlow(input)).resolves.toBeDefined();
  });

  it('should throw when gate require_access fails', async () => {
    const input = {
      role: '',
      verified: false,
      weightKg: 0.0,
      zone: '',
      priority: false,
      memberTier: '',
    };

    await expect(orderFlow(input)).rejects.toThrow();
  });

});

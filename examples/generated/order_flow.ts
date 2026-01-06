
// GENERATED FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.852326686+00:00
// DO NOT EDIT - regenerate from spec

export interface OrderFlowInput {
    role: string;
    verified: boolean;
    weightKg: number;
    zone: string;
    priority: boolean;
    memberTier: string;
}

export interface OrderFlowOutput {
    canOrder: boolean;
    shippingCost: number;
}

interface OrderFlowContext {
    check_access?: unknown;
    calc_shipping?: unknown;
}

export class OrderFlowError extends Error {
    constructor(
        public readonly step: string,
        public readonly type: "step_failed" | "gate_failed" | "timeout",
        message: string
    ) {
        super(message);
        this.name = "OrderFlowError";
    }
}

export async function orderFlow(input: OrderFlowInput): Promise<OrderFlowOutput> {
    const ctx: OrderFlowContext = {};
    // Step: check_access (call access_level)
    const check_access_result = await access_level({
        verified: input.verified,
        role: input.role
    });
    ctx.check_access = check_access_result;

    // Gate: require_access
    if (!(ctx.check_access?.level >= 50)) {
        throw new OrderFlowError(
            "require_access",
            "gate_failed",
            "Gate condition failed: check_access.level >= 50"
        );
    }
    // Step: calc_shipping (call shipping_rate)
    const calc_shipping_result = await shipping_rate({
        memberTier: input.member_tier,
        zone: input.zone,
        weightKg: input.weight_kg,
        priority: input.priority
    });
    ctx.calc_shipping = calc_shipping_result;

    return {
        canOrder: undefined as any, // TODO: map output from context
        shippingCost: undefined as any, // TODO: map output from context
    };
}
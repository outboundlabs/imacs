
// GENERATED FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.829917593+00:00
// DO NOT EDIT - regenerate from spec

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFlowInput {
    pub role: String,
    pub verified: bool,
    pub weight_kg: f64,
    pub zone: String,
    pub priority: bool,
    pub member_tier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFlowOutput {
    pub can_order: bool,
    pub shipping_cost: f64,
}

#[derive(Debug, Clone, Default)]
struct OrderFlowContext {
    check_access: Option<Value>,
    calc_shipping: Option<Value>,
}

#[derive(Debug, Clone)]
pub enum OrderFlowError {
    StepFailed { step: String, message: String },
    GateFailed { gate: String, condition: String },
    Timeout { step: String },
}

pub fn order_flow(input: OrderFlowInput) -> Result<OrderFlowOutput, OrderFlowError> {
    let mut ctx = OrderFlowContext::default();
    // Step: check_access (call access_level)
    let check_access_input = AccessLevelInput {
        verified: input.verified,
        role: input.role
    };
    let check_access_result = access_level(check_access_input);
    ctx.check_access = Some(serde_json::to_value(&check_access_result).unwrap());

    // Gate: require_access
    if !(ctx.check_access["level >= 50"]) {
        return Err(OrderFlowError::GateFailed {
            gate: "require_access".into(),
            condition: "check_access.level >= 50".into(),
        });
    }
    // Step: calc_shipping (call shipping_rate)
    let calc_shipping_input = ShippingRateInput {
        zone: input.zone,
        member_tier: input.member_tier,
        weight_kg: input.weight_kg,
        priority: input.priority
    };
    let calc_shipping_result = shipping_rate(calc_shipping_input);
    ctx.calc_shipping = Some(serde_json::to_value(&calc_shipping_result).unwrap());

    Ok(OrderFlowOutput {
        can_order: todo!("map output can_order from context"),
        shipping_cost: todo!("map output shipping_cost from context"),
    })
}
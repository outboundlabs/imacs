use serde::{Deserialize, Serialize}; use serde_json::Value; use crate::access_level::access_level;

use crate::shipping_rate::shipping_rate;

 #[derive(Debug, Clone, Serialize, Deserialize)] pub struct OrderFlowInput 
 { pub role: String,

pub verified: bool,

pub weight_kg: f64,

pub zone: String,

pub priority: bool,

pub member_tier: String, } 

#[derive(Debug, Clone, Serialize, Deserialize)] pub struct OrderFlowOutput { pub can_order: bool,

pub shipping_cost: f64, } #[derive(Debug, Clone, Default)] struct OrderFlowContext { check_access: Option<Value>,

require_access: Option<Value>,

calc_shipping: Option<Value>, } #[derive(Debug, Clone)] pub enum OrderFlowError { StepFailed { step: String, message: String }, GateFailed { gate: String, condition: String }, Timeout { step: String }, } pub fn order_flow(input: OrderFlowInput) -> Result<OrderFlowOutput, OrderFlowError> { let mut ctx = OrderFlowContext::default(); let check_access_input = AccessLevelInput { role: role,

verified: verified, }; let check_access_result = access_level(check_access_input); ctx.check_access = Some(serde_json::to_value(&check_access_result).unwrap());

if !(ctx.check_access.as_ref().unwrap()["level >= 50"]) { return Err(OrderFlowError::GateFailed { gate: require_access.into(), condition: "check_access.level >= 50", }); }

let calc_shipping_input = ShippingRateInput { member_tier: member_tier,

zone: zone,

priority: priority,

weight_kg: weight_kg, }; let calc_shipping_result = shipping_rate(calc_shipping_input); ctx.calc_shipping = Some(serde_json::to_value(&calc_shipping_result).unwrap()); Ok(OrderFlowOutput { can_order: todo!("map output"),

shipping_cost: todo!("map output"), }) }

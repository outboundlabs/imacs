import { access_level } from "./access_level"

import { shipping_rate } from "./shipping_rate"

 export interface OrderFlowInput { role: string

verified: boolean

weight_kg: number

zone: string

priority: boolean

member_tier: string } export interface OrderFlowOutput { can_order: boolean

shipping_cost: number } export async function order_flow(input: OrderFlowInput): Promise<OrderFlowOutput> { const ctx: Record<string, any> = {} ctx.check_access = await access_level({ role: input.role,

verified: input.verified, })

if (!(ctx.check_access?.level >= 50)) throw new Error("Gate require_access failed")

ctx.calc_shipping = await shipping_rate({ priority: input.priority,

zone: input.zone,

weight_kg: input.weight_kg,

member_tier: input.member_tier, }) return { can_order: undefined as any,

shipping_cost: undefined as any, } }

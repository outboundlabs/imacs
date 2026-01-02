using System; using System.Collections.Generic; using System.Threading.Tasks; using Newtonsoft.Json.Linq; namespace Generated { public class OrderFlowInput { public string role { get; set; }

public bool verified { get; set; }

public double weight_kg { get; set; }

public string zone { get; set; }

public bool priority { get; set; }

public string member_tier { get; set; } } public class OrderFlowOutput { public bool can_order { get; set; }

public double shipping_cost { get; set; } } public class OrderFlowContext { public JToken check_access { get; set; }

public JToken require_access { get; set; }

public JToken calc_shipping { get; set; } } public class OrderFlowException : Exception { public string Step { get; } public string Reason { get; } public OrderFlowException(string step, string reason) : base("Gate { step } failed: { reason }") { Step = step; Reason = reason; } } public static class OrderFlow { public static async Task<OrderFlowOutput> ExecuteAsync(OrderFlowInput input) { var ctx = new OrderFlowContext(); var check_accessInput = new AccessLevelInput { Verified = input.verified,

Role = input.role, }; var check_accessResult = await AccessLevel.ExecuteAsync(check_accessInput); ctx.check_access = JToken.FromObject(check_accessResult);

if (!(ctx.check_access["level >= 50"])) { throw new OrderFlowException("require_access", "check_access.level >= 50"); }

var calc_shippingInput = new ShippingRateInput { WeightKg = input.weight_kg,

MemberTier = input.member_tier,

Priority = input.priority,

Zone = input.zone, }; var calc_shippingResult = await ShippingRate.ExecuteAsync(calc_shippingInput); ctx.calc_shipping = JToken.FromObject(calc_shippingResult); return new OrderFlowOutput { can_order = default,

shipping_cost = default, }; } } }

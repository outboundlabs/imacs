package generated; import java.util.Map; import java.util.HashMap; import java.util.List; import java.util.concurrent.CompletableFuture; import com.fasterxml.jackson.databind.JsonNode; import com.fasterxml.jackson.databind.ObjectMapper; public class OrderFlow { private static final ObjectMapper mapper = new ObjectMapper(); public static class Input { public String role;

public Boolean verified;

public Double weight_kg;

public String zone;

public Boolean priority;

public String member_tier; public Input() {} } public static class Output { public Boolean can_order;

public Double shipping_cost; public Output() {} } public static class Context { public JsonNode check_access;

public JsonNode require_access;

public JsonNode calc_shipping; } public static class OrchestrationException extends RuntimeException { public final String step; public final String reason; public OrchestrationException(String step, String reason) { super("Gate " + step + " failed: " + reason); this.step = step; this.reason = reason; } } public static CompletableFuture<Output> executeAsync(Input input) { return CompletableFuture.supplyAsync(() -> { try { return execute(input); } catch (Exception e) { throw new RuntimeException(e); } }); } public static Output execute(Input input) throws Exception { Context ctx = new Context(); var check_accessInput = new AccessLevel.Input(); check_accessInput.role = input.role;

check_accessInput.verified = input.verified; var check_accessResult = AccessLevel.execute(check_accessInput); ctx.check_access = mapper.valueToTree(check_accessResult);

if (!(ctx.check_access.get("level >= 50"))) { throw new OrchestrationException("require_access", "check_access.level >= 50"); }

var calc_shippingInput = new ShippingRate.Input(); calc_shippingInput.zone = input.zone;

calc_shippingInput.priority = input.priority;

calc_shippingInput.member_tier = input.member_tier;

calc_shippingInput.weight_kg = input.weight_kg; var calc_shippingResult = ShippingRate.execute(calc_shippingInput); ctx.calc_shipping = mapper.valueToTree(calc_shippingResult); Output output = new Output(); output.can_order = null;

output.shipping_cost = null; return output; } }

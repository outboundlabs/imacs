


// GENERATED FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.944911336+00:00
// DO NOT EDIT - regenerate from spec


import java.util.*;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;

public class OrderFlowOrchestrator {
    private static final ObjectMapper mapper = new ObjectMapper();

    public static class Input {

        public String role;

        public boolean verified;

        public double weightKg;

        public String zone;

        public boolean priority;

        public String memberTier;


        public Input(String role, boolean verified, double weightKg, String zone, boolean priority, String memberTier) {

            this.role = role;

            this.verified = verified;

            this.weightKg = weightKg;

            this.zone = zone;

            this.priority = priority;

            this.memberTier = memberTier;

        }
    }

    public static class Output {

        public boolean canOrder;

        public double shippingCost;


        public Output(boolean canOrder, double shippingCost) {

            this.canOrder = canOrder;

            this.shippingCost = shippingCost;

        }
    }

    public static class Context {


        public JsonNode check_access;





        public JsonNode calc_shipping;


    }

    public static class OrderFlowException extends RuntimeException {
        public final String step;
        public final String type;

        public OrderFlowException(String step, String type, String message) {
            super(message);
            this.step = step;
            this.type = type;
        }
    }

    public static Output execute(Input input) {
        Context ctx = new Context();



        // Step: check_access (call access_level)
        var check_accessInput = new AccessLevel.Input(

            input.verified,

            input.role

        );
        var check_accessResult = AccessLevel.evaluate(check_accessInput);
        ctx.check_access = mapper.valueToTree(check_accessResult);





        // Gate: require_access
        if (!(ctx.checkAccess.get("level >= 50"))) {
            throw new OrderFlowException(
                "require_access",
                "gate_failed",
                "Gate condition failed: check_access.level >= 50"
            );
        }




        // Step: calc_shipping (call shipping_rate)
        var calc_shippingInput = new ShippingRate.Input(

            input.memberTier,

            input.zone,

            input.priority,

            input.weightKg

        );
        var calc_shippingResult = ShippingRate.evaluate(calc_shippingInput);
        ctx.calc_shipping = mapper.valueToTree(calc_shippingResult);




        return new Output(

            null,  // TODO: map output from context

            null  // TODO: map output from context

        );
    }
}
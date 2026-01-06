

// GENERATED FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.897106482+00:00
// DO NOT EDIT - regenerate from spec


using System;
using System.Collections.Generic;
using Newtonsoft.Json.Linq;

namespace none
{
    public class OrderFlowInput
    {

        public string Role { get; set; }

        public bool Verified { get; set; }

        public double WeightKg { get; set; }

        public string Zone { get; set; }

        public bool Priority { get; set; }

        public string MemberTier { get; set; }

    }

    public class OrderFlowOutput
    {

        public bool CanOrder { get; set; }

        public double ShippingCost { get; set; }

    }

    internal class OrderFlowContext
    {


        public JToken check_access;





        public JToken calc_shipping;


    }

    public class OrderFlowException : Exception
    {
        public string Step { get; }
        public string ErrorType { get; }

        public OrderFlowException(string step, string errorType, string message)
            : base(message)
        {
            Step = step;
            ErrorType = errorType;
        }
    }

    public static class OrderFlowOrchestrator
    {
        public static OrderFlowOutput Execute(OrderFlowInput input)
        {
            var ctx = new OrderFlowContext();



            // Step: check_access (call access_level)
            var check_accessInput = new AccessLevelInput
            {

                Verified = input.Verified,

                Role = input.Role

            };
            var check_accessResult = AccessLevel.Evaluate(check_accessInput);
            ctx.check_access = JToken.FromObject(check_accessResult);





            // Gate: require_access
            if (!(ctx.check_access["level >= 50"]))
            {
                throw new OrderFlowException(
                    "require_access",
                    "gate_failed",
                    "Gate condition failed: check_access.level >= 50"
                );
            }




            // Step: calc_shipping (call shipping_rate)
            var calc_shippingInput = new ShippingRateInput
            {

                MemberTier = input.MemberTier,

                WeightKg = input.WeightKg,

                Priority = input.Priority,

                Zone = input.Zone

            };
            var calc_shippingResult = ShippingRate.Evaluate(calc_shippingInput);
            ctx.calc_shipping = JToken.FromObject(calc_shippingResult);




            return new OrderFlowOutput
            {

                CanOrder = default,  // TODO: map output from context

                ShippingCost = default,  // TODO: map output from context

            };
        }
    }
}
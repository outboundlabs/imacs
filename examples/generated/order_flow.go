

// GENERATED FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.924264889+00:00
// DO NOT EDIT - regenerate from spec



package none

import (
	"encoding/json"
	"fmt"
)

type OrderFlowInput struct {

	Role string `json:"role"`

	Verified bool `json:"verified"`

	WeightKg float64 `json:"weight_kg"`

	Zone string `json:"zone"`

	Priority bool `json:"priority"`

	MemberTier string `json:"member_tier"`

}

type OrderFlowOutput struct {

	CanOrder bool `json:"can_order"`

	ShippingCost float64 `json:"shipping_cost"`

}

type OrderFlowContext struct {


	CheckAccess interface{}





	CalcShipping interface{}


}

type OrderFlowError struct {
	Step    string
	Type    string
	Message string
}

func (e OrderFlowError) Error() string {
	return fmt.Sprintf("%s error in step %s: %s", e.Type, e.Step, e.Message)
}

func OrderFlow(input OrderFlowInput) (OrderFlowOutput, error) {
	ctx := OrderFlowContext{}



	// Step: check_access (call access_level)
	check_accessInput := AccessLevelInput{

		Role: input.Role,

		Verified: input.Verified

	}
	check_accessResult := AccessLevel(check_accessInput)
	ctx.CheckAccess = check_accessResult





	// Gate: require_access
	if !(ctx.CheckAccess["level >= 50"]) {
		return OrderFlowOutput{}, OrderFlowError{
			Step:    "require_access",
			Type:    "gate_failed",
			Message: "Gate condition failed: check_access.level >= 50",
		}
	}




	// Step: calc_shipping (call shipping_rate)
	calc_shippingInput := ShippingRateInput{

		Zone: input.Zone,

		WeightKg: input.WeightKg,

		Priority: input.Priority,

		MemberTier: input.MemberTier

	}
	calc_shippingResult := ShippingRate(calc_shippingInput)
	ctx.CalcShipping = calc_shippingResult




	return OrderFlowOutput{

		CanOrder: /* TODO: map output from context */,

		ShippingCost: /* TODO: map output from context */,

	}, nil
}
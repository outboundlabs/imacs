package generated import ( "fmt" "sync" ) type OrderFlowInput struct { Role string

Verified bool

WeightKg float64

Zone string

Priority bool

MemberTier string } type OrderFlowOutput struct { CanOrder bool

ShippingCost float64 } type OrderFlowContext struct { CheckAccess interface{}

RequireAccess interface{}

CalcShipping interface{} } type OrderFlowError struct { Step string Reason string } func (e *OrderFlowError) Error() string { return fmt.Sprintf("gate %s failed: %s", e.Step, e.Reason) } func OrderFlowExecute(input *OrderFlowInput) (*OrderFlowOutput, error) { ctx := &OrderFlowContext{} check_accessInput := &AccessLevelInput{ Role: input.Role,

Verified: input.Verified, } check_accessResult, err := AccessLevelExecute(check_accessInput) if err != nil { return nil, fmt.Errorf("step $(call_id) failed: %w", err) } ctx.CheckAccess = check_accessResult

if !(ctx.CheckAccess["level >= 50"]) { return nil, &OrderFlowError{Step: "require_access", Reason: "check_access.level >= 50"} }

calc_shippingInput := &ShippingRateInput{ WeightKg: input.WeightKg,

Zone: input.Zone,

Priority: input.Priority,

MemberTier: input.MemberTier, } calc_shippingResult, err := ShippingRateExecute(calc_shippingInput) if err != nil { return nil, fmt.Errorf("step $(call_id) failed: %w", err) } ctx.CalcShipping = calc_shippingResult output := &OrderFlowOutput{ CanOrder: nil,

ShippingCost: nil, } return output, nil }

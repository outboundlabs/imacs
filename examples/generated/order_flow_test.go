// GENERATED TESTS FROM: order_flow.yaml
// GENERATED: 2026-01-05T17:28:46.924390690+00:00
// DO NOT EDIT — regenerate from spec

package main

import (
	"testing"
)

func TestOrderFlow_HappyPath(t *testing.T) {
	input := OrderFlowInput{
		Role: "test",
		Verified: true,
		WeightKg: 10.0,
		Zone: "test",
		Priority: true,
		MemberTier: "test",
	}

	_, err := OrderFlow(input)
	if err != nil {
		t.Errorf("expected success, got error: %v", err)
	}
}

func TestOrderFlow_Gate_RequireAccess_Fails(t *testing.T) {
	input := OrderFlowInput{
		Role: "",
		Verified: false,
		WeightKg: 0.0,
		Zone: "",
		Priority: false,
		MemberTier: "",
	}

	_, err := OrderFlow(input)
	if err == nil {
		t.Error("expected error, got success")
	}
	if orchErr, ok := err.(OrderFlowError); ok {
		if orchErr.Step != "require_access" {
			t.Errorf("expected step 'require_access', got '%s'", orchErr.Step)
		}
	}
}


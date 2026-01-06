// GENERATED TESTS FROM: shipping_rate.yaml
// SPEC HASH: sha256:bfd80b5a15c6208e
// GENERATED: 2026-01-05T17:28:47.541289348+00:00
// DO NOT EDIT — regenerate from spec

package main

import "testing"

func TestShippingRate_R1(t *testing.T) {
	// R1: member_tier == 'gold' && zone == 'domestic' → 0
	input := ShippingRateInput{WeightKg: 0.0, Zone: "domestic", Priority: false, MemberTier: "gold"}
	result := ShippingRate(input)
	if result != 0 {
		t.Errorf("Expected 0, got %v", result)
	}
}

func TestShippingRate_R2(t *testing.T) {
	// R2: priority && zone == 'international' → "weight_kg * 25.0 + 50.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "international", Priority: true, MemberTier: ""}
	result := ShippingRate(input)
	if result != "weight_kg * 25.0 + 50.0" {
		t.Errorf("Expected "weight_kg * 25.0 + 50.0", got %v", result)
	}
}

func TestShippingRate_R3(t *testing.T) {
	// R3: priority && zone == 'north_america' → "weight_kg * 15.0 + 20.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "north_america", Priority: true, MemberTier: ""}
	result := ShippingRate(input)
	if result != "weight_kg * 15.0 + 20.0" {
		t.Errorf("Expected "weight_kg * 15.0 + 20.0", got %v", result)
	}
}

func TestShippingRate_R4(t *testing.T) {
	// R4: priority && zone == 'domestic' → "weight_kg * 8.0 + 10.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "domestic", Priority: true, MemberTier: ""}
	result := ShippingRate(input)
	if result != "weight_kg * 8.0 + 10.0" {
		t.Errorf("Expected "weight_kg * 8.0 + 10.0", got %v", result)
	}
}

func TestShippingRate_R5(t *testing.T) {
	// R5: member_tier == 'silver' && zone == 'international' → "weight_kg * 16.0 + 30.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "international", Priority: false, MemberTier: "silver"}
	result := ShippingRate(input)
	if result != "weight_kg * 16.0 + 30.0" {
		t.Errorf("Expected "weight_kg * 16.0 + 30.0", got %v", result)
	}
}

func TestShippingRate_R6(t *testing.T) {
	// R6: member_tier == 'silver' && zone == 'north_america' → "weight_kg * 8.0 + 12.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "north_america", Priority: false, MemberTier: "silver"}
	result := ShippingRate(input)
	if result != "weight_kg * 8.0 + 12.0" {
		t.Errorf("Expected "weight_kg * 8.0 + 12.0", got %v", result)
	}
}

func TestShippingRate_R7(t *testing.T) {
	// R7: member_tier == 'silver' && zone == 'domestic' → "weight_kg * 4.0 + 5.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "domestic", Priority: false, MemberTier: "silver"}
	result := ShippingRate(input)
	if result != "weight_kg * 4.0 + 5.0" {
		t.Errorf("Expected "weight_kg * 4.0 + 5.0", got %v", result)
	}
}

func TestShippingRate_R8(t *testing.T) {
	// R8: zone == 'international' → "weight_kg * 20.0 + 40.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "international", Priority: false, MemberTier: ""}
	result := ShippingRate(input)
	if result != "weight_kg * 20.0 + 40.0" {
		t.Errorf("Expected "weight_kg * 20.0 + 40.0", got %v", result)
	}
}

func TestShippingRate_R9(t *testing.T) {
	// R9: zone == 'north_america' → "weight_kg * 10.0 + 15.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "north_america", Priority: false, MemberTier: ""}
	result := ShippingRate(input)
	if result != "weight_kg * 10.0 + 15.0" {
		t.Errorf("Expected "weight_kg * 10.0 + 15.0", got %v", result)
	}
}

func TestShippingRate_R10(t *testing.T) {
	// R10: zone == 'domestic' → "weight_kg * 5.0 + 7.0"
	input := ShippingRateInput{WeightKg: 0.0, Zone: "domestic", Priority: false, MemberTier: ""}
	result := ShippingRate(input)
	if result != "weight_kg * 5.0 + 7.0" {
		t.Errorf("Expected "weight_kg * 5.0 + 7.0", got %v", result)
	}
}


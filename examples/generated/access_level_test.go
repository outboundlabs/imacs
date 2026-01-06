// GENERATED TESTS FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.746261451+00:00
// DO NOT EDIT — regenerate from spec

package main

import "testing"

func TestAccessLevel_R1(t *testing.T) {
	// R1: role == 'admin' → 100
	input := AccessLevelInput{Role: "admin", Verified: false}
	result := AccessLevel(input)
	if result != 100 {
		t.Errorf("Expected 100, got %v", result)
	}
}

func TestAccessLevel_R2(t *testing.T) {
	// R2: role == 'member' && verified → 50
	input := AccessLevelInput{Role: "member", Verified: true}
	result := AccessLevel(input)
	if result != 50 {
		t.Errorf("Expected 50, got %v", result)
	}
}

func TestAccessLevel_R3(t *testing.T) {
	// R3: role == 'member' && !verified → 25
	input := AccessLevelInput{Role: "member", Verified: false}
	result := AccessLevel(input)
	if result != 25 {
		t.Errorf("Expected 25, got %v", result)
	}
}

func TestAccessLevel_R4(t *testing.T) {
	// R4: role == 'guest' → 10
	input := AccessLevelInput{Role: "guest", Verified: false}
	result := AccessLevel(input)
	if result != 10 {
		t.Errorf("Expected 10, got %v", result)
	}
}


// GENERATED FROM: shipping_rate.yaml // SPEC HASH: sha256:bfd80b5a15c6208e // GENERATED: 2026-01-02T14:50:57.346675101+00:00

 public class ShippingRateInput { public double WeightKg { get; set; }

public string Zone { get; set; }

public bool Priority { get; set; }

public string MemberTier { get; set; } }

 public static class ShippingRate { public static double Evaluate(ShippingRateInput input) { var weightKg = input.WeightKg;
var zone = input.Zone;
var priority = input.Priority;
var memberTier = input.MemberTier;

 if (((member_tier == "gold") && (zone == "domestic"))) { // R1

 return 0d; }else if ((priority && (zone == "international"))) { // R2

 return "weight_kg * 25.0 + 50.0"; }else if ((priority && (zone == "north_america"))) { // R3

 return "weight_kg * 15.0 + 20.0"; }else if ((priority && (zone == "domestic"))) { // R4

 return "weight_kg * 8.0 + 10.0"; }else if (((member_tier == "silver") && (zone == "international"))) { // R5

 return "weight_kg * 16.0 + 30.0"; }else if (((member_tier == "silver") && (zone == "north_america"))) { // R6

 return "weight_kg * 8.0 + 12.0"; }else if (((member_tier == "silver") && (zone == "domestic"))) { // R7

 return "weight_kg * 4.0 + 5.0"; }else if ((zone == "international")) { // R8

 return "weight_kg * 20.0 + 40.0"; }else if ((zone == "north_america")) { // R9

 return "weight_kg * 10.0 + 15.0"; }else if ((zone == "domestic")) { // R10

 return "weight_kg * 5.0 + 7.0"; } } }

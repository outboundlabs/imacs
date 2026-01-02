// GENERATED FROM: shipping_rate.yaml // SPEC HASH: sha256:bfd80b5a15c6208e // GENERATED: 2026-01-02T15:13:56.245555424+00:00

 public class ShippingRate {

 public static class ShippingRateInput { public double weightKg;

public String zone;

public boolean priority;

public String memberTier; }

 public static double evaluate(ShippingRateInput input) { if (((input.memberTier == "gold") && (input.zone == "domestic"))) { // R1

 return 0d; }else if ((input.priority && (input.zone == "international"))) { // R2

 return ((input.weightKg * 25.0) + 50.0); }else if ((input.priority && (input.zone == "north_america"))) { // R3

 return ((input.weightKg * 15.0) + 20.0); }else if ((input.priority && (input.zone == "domestic"))) { // R4

 return ((input.weightKg * 8.0) + 10.0); }else if (((input.memberTier == "silver") && (input.zone == "international"))) { // R5

 return ((input.weightKg * 16.0) + 30.0); }else if (((input.memberTier == "silver") && (input.zone == "north_america"))) { // R6

 return ((input.weightKg * 8.0) + 12.0); }else if (((input.memberTier == "silver") && (input.zone == "domestic"))) { // R7

 return ((input.weightKg * 4.0) + 5.0); }else if ((input.zone == "international")) { // R8

 return ((input.weightKg * 20.0) + 40.0); }else if ((input.zone == "north_america")) { // R9

 return ((input.weightKg * 10.0) + 15.0); }else if ((input.zone == "domestic")) { // R10

 return ((input.weightKg * 5.0) + 7.0); } } }


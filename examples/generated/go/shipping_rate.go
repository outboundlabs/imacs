// GENERATED FROM: shipping_rate.yaml // SPEC HASH: sha256:bfd80b5a15c6208e // GENERATED: 2026-01-02T15:13:55.178933734+00:00

 package shipping_rate

 type ShippingRateInput struct { WeightKg float64

Zone string

Priority bool

MemberTier string }

 func ShippingRate(input ShippingRateInput) float64 { if ((input.MemberTier == "gold") && (input.Zone == "domestic")) { // R1

 return 0 }else if (input.Priority && (input.Zone == "international")) { // R2

 return ((input.WeightKg * 25.0) + 50.0) }else if (input.Priority && (input.Zone == "north_america")) { // R3

 return ((input.WeightKg * 15.0) + 20.0) }else if (input.Priority && (input.Zone == "domestic")) { // R4

 return ((input.WeightKg * 8.0) + 10.0) }else if ((input.MemberTier == "silver") && (input.Zone == "international")) { // R5

 return ((input.WeightKg * 16.0) + 30.0) }else if ((input.MemberTier == "silver") && (input.Zone == "north_america")) { // R6

 return ((input.WeightKg * 8.0) + 12.0) }else if ((input.MemberTier == "silver") && (input.Zone == "domestic")) { // R7

 return ((input.WeightKg * 4.0) + 5.0) }else if (input.Zone == "international") { // R8

 return ((input.WeightKg * 20.0) + 40.0) }else if (input.Zone == "north_america") { // R9

 return ((input.WeightKg * 10.0) + 15.0) }else if (input.Zone == "domestic") { // R10

 return ((input.WeightKg * 5.0) + 7.0) } }


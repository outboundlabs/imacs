// GENERATED TESTS FROM: shipping_rate.yaml
// SPEC HASH: sha256:bfd80b5a15c6208e
// GENERATED: 2026-01-05T17:28:47.423230586+00:00
// DO NOT EDIT — regenerate from spec

using Xunit;

public class ShippingRateTests
{
    [Fact]
    public void Test_R1()
    {
        // R1: member_tier == 'gold' && zone == 'domestic' → 0
        Assert.Equal(0d, ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "domestic", Priority = false, MemberTier = "gold" }));
    }

    [Fact]
    public void Test_R2()
    {
        // R2: priority && zone == 'international' → "weight_kg * 25.0 + 50.0"
        Assert.Equal("weight_kg * 25.0 + 50.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "international", Priority = true, MemberTier = "" }));
    }

    [Fact]
    public void Test_R3()
    {
        // R3: priority && zone == 'north_america' → "weight_kg * 15.0 + 20.0"
        Assert.Equal("weight_kg * 15.0 + 20.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "north_america", Priority = true, MemberTier = "" }));
    }

    [Fact]
    public void Test_R4()
    {
        // R4: priority && zone == 'domestic' → "weight_kg * 8.0 + 10.0"
        Assert.Equal("weight_kg * 8.0 + 10.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "domestic", Priority = true, MemberTier = "" }));
    }

    [Fact]
    public void Test_R5()
    {
        // R5: member_tier == 'silver' && zone == 'international' → "weight_kg * 16.0 + 30.0"
        Assert.Equal("weight_kg * 16.0 + 30.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "international", Priority = false, MemberTier = "silver" }));
    }

    [Fact]
    public void Test_R6()
    {
        // R6: member_tier == 'silver' && zone == 'north_america' → "weight_kg * 8.0 + 12.0"
        Assert.Equal("weight_kg * 8.0 + 12.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "north_america", Priority = false, MemberTier = "silver" }));
    }

    [Fact]
    public void Test_R7()
    {
        // R7: member_tier == 'silver' && zone == 'domestic' → "weight_kg * 4.0 + 5.0"
        Assert.Equal("weight_kg * 4.0 + 5.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "domestic", Priority = false, MemberTier = "silver" }));
    }

    [Fact]
    public void Test_R8()
    {
        // R8: zone == 'international' → "weight_kg * 20.0 + 40.0"
        Assert.Equal("weight_kg * 20.0 + 40.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "international", Priority = false, MemberTier = "" }));
    }

    [Fact]
    public void Test_R9()
    {
        // R9: zone == 'north_america' → "weight_kg * 10.0 + 15.0"
        Assert.Equal("weight_kg * 10.0 + 15.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "north_america", Priority = false, MemberTier = "" }));
    }

    [Fact]
    public void Test_R10()
    {
        // R10: zone == 'domestic' → "weight_kg * 5.0 + 7.0"
        Assert.Equal("weight_kg * 5.0 + 7.0", ShippingRate.Evaluate(new ShippingRateInput { WeightKg = 0.0, Zone = "domestic", Priority = false, MemberTier = "" }));
    }

}

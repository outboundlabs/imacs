// GENERATED TESTS FROM: shipping_rate.yaml
// SPEC HASH: sha256:bfd80b5a15c6208e
// GENERATED: 2026-01-05T17:28:47.660440210+00:00
// DO NOT EDIT — regenerate from spec

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

public class ShippingRateTest {
    @Test
    public void testR1() {
        // R1: member_tier == 'gold' && zone == 'domestic' → 0
        var input = new ShippingRate.Input(0.0, "domestic", false, "gold");
        assertEquals(0d, ShippingRate.evaluate(input));
    }

    @Test
    public void testR2() {
        // R2: priority && zone == 'international' → "weight_kg * 25.0 + 50.0"
        var input = new ShippingRate.Input(0.0, "international", true, "");
        assertEquals("weight_kg * 25.0 + 50.0", ShippingRate.evaluate(input));
    }

    @Test
    public void testR3() {
        // R3: priority && zone == 'north_america' → "weight_kg * 15.0 + 20.0"
        var input = new ShippingRate.Input(0.0, "north_america", true, "");
        assertEquals("weight_kg * 15.0 + 20.0", ShippingRate.evaluate(input));
    }

    @Test
    public void testR4() {
        // R4: priority && zone == 'domestic' → "weight_kg * 8.0 + 10.0"
        var input = new ShippingRate.Input(0.0, "domestic", true, "");
        assertEquals("weight_kg * 8.0 + 10.0", ShippingRate.evaluate(input));
    }

    @Test
    public void testR5() {
        // R5: member_tier == 'silver' && zone == 'international' → "weight_kg * 16.0 + 30.0"
        var input = new ShippingRate.Input(0.0, "international", false, "silver");
        assertEquals("weight_kg * 16.0 + 30.0", ShippingRate.evaluate(input));
    }

    @Test
    public void testR6() {
        // R6: member_tier == 'silver' && zone == 'north_america' → "weight_kg * 8.0 + 12.0"
        var input = new ShippingRate.Input(0.0, "north_america", false, "silver");
        assertEquals("weight_kg * 8.0 + 12.0", ShippingRate.evaluate(input));
    }

    @Test
    public void testR7() {
        // R7: member_tier == 'silver' && zone == 'domestic' → "weight_kg * 4.0 + 5.0"
        var input = new ShippingRate.Input(0.0, "domestic", false, "silver");
        assertEquals("weight_kg * 4.0 + 5.0", ShippingRate.evaluate(input));
    }

    @Test
    public void testR8() {
        // R8: zone == 'international' → "weight_kg * 20.0 + 40.0"
        var input = new ShippingRate.Input(0.0, "international", false, "");
        assertEquals("weight_kg * 20.0 + 40.0", ShippingRate.evaluate(input));
    }

    @Test
    public void testR9() {
        // R9: zone == 'north_america' → "weight_kg * 10.0 + 15.0"
        var input = new ShippingRate.Input(0.0, "north_america", false, "");
        assertEquals("weight_kg * 10.0 + 15.0", ShippingRate.evaluate(input));
    }

    @Test
    public void testR10() {
        // R10: zone == 'domestic' → "weight_kg * 5.0 + 7.0"
        var input = new ShippingRate.Input(0.0, "domestic", false, "");
        assertEquals("weight_kg * 5.0 + 7.0", ShippingRate.evaluate(input));
    }

}

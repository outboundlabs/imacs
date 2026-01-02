// GENERATED FROM: shipping_rate.yaml
// SPEC HASH: sha256:bfd80b5a15c6208e
// GENERATED: 2026-01-02T15:13:54.122643401+00:00
// DO NOT EDIT — regenerate from spec

export interface ShippingRateInput {
    weightKg: number;
    zone: string;
    priority: boolean;
    memberTier: string;
}

export function shippingRate(input: ShippingRateInput): number {
    const { weightKg, zone, priority, memberTier } = input;

    if (((memberTier === "gold") && (zone === "domestic"))) {
        // R1
        return 0;
    } else if ((priority && (zone === "international"))) {
        // R2
        return ((weightKg * 25.0) + 50.0);
    } else if ((priority && (zone === "north_america"))) {
        // R3
        return ((weightKg * 15.0) + 20.0);
    } else if ((priority && (zone === "domestic"))) {
        // R4
        return ((weightKg * 8.0) + 10.0);
    } else if (((memberTier === "silver") && (zone === "international"))) {
        // R5
        return ((weightKg * 16.0) + 30.0);
    } else if (((memberTier === "silver") && (zone === "north_america"))) {
        // R6
        return ((weightKg * 8.0) + 12.0);
    } else if (((memberTier === "silver") && (zone === "domestic"))) {
        // R7
        return ((weightKg * 4.0) + 5.0);
    } else if ((zone === "international")) {
        // R8
        return ((weightKg * 20.0) + 40.0);
    } else if ((zone === "north_america")) {
        // R9
        return ((weightKg * 10.0) + 15.0);
    } else if ((zone === "domestic")) {
        // R10
        return ((weightKg * 5.0) + 7.0);
    }
}


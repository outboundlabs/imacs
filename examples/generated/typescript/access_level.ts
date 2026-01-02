// GENERATED FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-02T15:13:54.400722586+00:00
// DO NOT EDIT — regenerate from spec

export interface AccessLevelInput {
    role: string;
    verified: boolean;
}

export function accessLevel(input: AccessLevelInput): number {
    const { role, verified } = input;

    if ((role === "admin")) {
        // R1
        return 100;
    } else if (((role === "member") && verified)) {
        // R2
        return 50;
    } else if (((role === "member") && (!verified))) {
        // R3
        return 25;
    } else if ((role === "guest")) {
        // R4
        return 10;
    }
}


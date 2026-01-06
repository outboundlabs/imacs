

// GENERATED FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.699260198+00:00
// DO NOT EDIT - regenerate from spec


using System;
using System.Collections.Generic;


public class AccessLevelInput
{

    public string Role { get; set; }

    public bool Verified { get; set; }

}


public static class AccessLevel
{
    public static long Evaluate(AccessLevelInput input)
    {

        var role = input.Role;

        var verified = input.Verified;




        if ((role == "admin"))
        {

            // R1
            return 100L;


        }
        else if (((role == "member") && verified))
        {

            // R2
            return 50L;


        }
        else if (((role == "member") && (!verified)))
        {

            // R3
            return 25L;


        }
        else if ((role == "guest"))
        {

            // R4
            return 10L;

        }
        else
        {

            throw new InvalidOperationException("No rule matched");

        }
    }
}



// GENERATED FROM: access_level.yaml
// SPEC HASH: sha256:61f180f99fb26ed2
// GENERATED: 2026-01-05T17:28:46.746168669+00:00
// DO NOT EDIT - regenerate from spec



package none

type AccessLevelInput struct {

	Role string `json:"role"`

	Verified bool `json:"verified"`

}


func AccessLevel(input AccessLevelInput) int64 {


	if (input.Role == "admin") {

		// R1
		return int64(100)


	} else if ((input.Role == "member") && input.Verified) {

		// R2
		return int64(50)


	} else if ((input.Role == "member") && (!input.Verified)) {

		// R3
		return int64(25)


	} else if (input.Role == "guest") {

		// R4
		return int64(10)

	} else {

		panic("No rule matched")

	}
}
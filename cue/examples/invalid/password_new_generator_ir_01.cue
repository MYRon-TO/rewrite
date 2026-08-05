package rewrite

ir_version: "0.1"
generators: bad: {
	type: "machine_secret"
	params: {profile: "uuid"}
}
scenario: {id: "s", systems: ["s"], instance_groups: ["g"], time_profile: "t"}

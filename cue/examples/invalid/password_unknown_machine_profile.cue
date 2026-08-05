package rewrite

ir_version: "0.2"
fixture_constraint: #MachineSecretParams & {profile: "not_a_profile"}
generators: bad: {
	type: "machine_secret"
	params: #MachineSecretParams & {profile: "not_a_profile"}
}
scenario: {id: "s", systems: ["s"], instance_groups: ["g"], time_profile: "t"}

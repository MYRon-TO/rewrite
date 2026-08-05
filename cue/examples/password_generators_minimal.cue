package rewrite

ir_version: "0.2"
generators: {
	secret: {
		type: "machine_secret"
		params: {profile: "uuid"}
	}
	password: {
		type: "corpus_choice"
		params: source: {builtin: "human-passwords-v1"}
	}
}
scenario: {id: "minimal", systems: ["system"], instance_groups: ["group"], time_profile: "default"}

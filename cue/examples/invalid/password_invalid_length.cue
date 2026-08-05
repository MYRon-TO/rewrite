package rewrite

ir_version: "0.2"
fixture_constraint: #CorpusChoiceParams & {
	source: {builtin: "human-passwords-v1"}
	min_length: 20
	max_length: 10
}
generators: bad: {
	type: "corpus_choice"
	params: #CorpusChoiceParams & {
		source: {builtin: "human-passwords-v1"}
		min_length: 20
		max_length: 10
	}
}
scenario: {id: "s", systems: ["s"], instance_groups: ["g"], time_profile: "t"}

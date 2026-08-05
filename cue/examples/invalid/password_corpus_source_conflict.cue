package rewrite

ir_version: "0.2"
fixture_constraint: #CorpusSource & {builtin: "human-passwords-v1", path: "passwords.txt"}
generators: bad: {
	type: "corpus_choice"
	params: #CorpusChoiceParams & {source: {builtin: "human-passwords-v1", path: "passwords.txt"}}
}
scenario: {id: "s", systems: ["s"], instance_groups: ["g"], time_profile: "t"}

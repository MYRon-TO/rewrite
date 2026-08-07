package rewrite

// A single weighted decision keeps the requested 60/20/20 mix approximate and transparent.
machines: secret_exposure_flow: {
	system: "credential_app"
	start:  "choose_sample"
	attack: {
		enabled:     true
		type:        "secret_exposure"
		technique:   "T1552"
		tactic:      "credential_access"
		description: "Synthetic secret written to a log"
	}
	states: {
		choose_sample: {
			kind: "benign"
			transitions: [
				{to: "ordinary_activity", weight: 60},
				{to: "hard_negative", weight: 20},
				{to: "choose_exposure", weight: 20},
			]
		}
		ordinary_activity: {
			kind: "benign"
			emit: [
				{template: "login_request"},
				{template: "login_verify"},
				{template: "login_success"},
				{template: "task_complete"},
			]
			terminal: true
		}
		hard_negative: {
			kind: "benign"
			transitions: [
				{to: "keyword_negative", weight: 1},
				{to: "entropy_negative", weight: 1},
				{to: "lookalike_negative", weight: 1},
			]
		}
		keyword_negative: {
			kind: "benign"
			emit: [
				{template: "password_placeholder"},
				{template: "secret_redacted"},
				{template: "password_expired"},
				{template: "key_rotation"},
			]
			terminal: true
		}
		entropy_negative: {
			kind: "benign"
			emit: [
				{template: "request_complete"},
				{template: "trace_success"},
				{template: "commit_deployed"},
				{template: "image_synced"},
			]
			terminal: true
		}
		lookalike_negative: {
			kind: "benign"
			emit: [
				{template: "version_release"},
				{template: "resource_created"},
				{template: "token_cache_clear"},
				{template: "api_success"},
			]
			terminal: true
		}
		choose_exposure: {
			kind: "benign"
			transitions: [
				{to: "variable_exposure", weight: 1},
				{to: "nested_exposure", weight: 1},
			]
		}
		variable_exposure: {
			kind: "attack"
			attack_label: {
				type:      "secret_exposure"
				technique: "T1552"
				tactic:    "credential_access"
				name:      "variable_secret_exposure"
			}
			emit: [
				{template: "password_exposed"},
				{template: "api_key_exposed"},
				{template: "bearer_exposed"},
				{template: "cloud_secret_exposed"},
			]
			terminal: true
		}
		nested_exposure: {
			kind: "attack"
			attack_label: {
				type:      "secret_exposure"
				technique: "T1552"
				tactic:    "credential_access"
				name:      "nested_secret_exposure"
			}
			emit: [
				{template: "password_nested"},
				{template: "api_key_nested"},
				{template: "bearer_nested"},
				{template: "cloud_pair_nested"},
			]
			terminal: true
		}
	}
}

package rewrite

systems: credential_app: {
	kind:        "application"
	description: "Application that authenticates users and calls external services"
}

generators: {
	username: {type: "username", params: {}}
	human_password: {
		type: "corpus_choice"
		params: {
			source: {builtin: "human-passwords-v1"}
			min_length: 6
			max_length: 64
			sampling:   "uniform"
		}
	}
	api_key: {
		type: "machine_secret"
		params: {profile: "api_key", prefix: "sim_sk_", body_length: 32, encoding: "base64url"}
	}
	bearer_token: {
		type: "machine_secret"
		params: {profile: "bearer_token", prefix: "sim_bearer_", body_length: 40, encoding: "base64url"}
	}
	cloud_pair: {
		type: "machine_secret"
		params: {profile: "cloud_secret_pair", id_prefix: "SIMAKIA", id_body_length: 16, secret_length: 40}
	}
	host_sequence: {
		type: "sequence"
		params: {prefix: "user-host-", start: 1, step: 1, pad: 3}
	}
	request_id: {type: "uuid", params: {}}
	trace_id: {type: "machine_secret", params: {profile: "hex", byte_length: 16}}
	session_id: {type: "uuid", params: {}}
	version: {type: "choice", params: {values: ["1.8.7", "2.4.12", "2026.05.17"]}}
	git_commit: {type: "choice", params: {values: ["9f86d081884c7d659a2feaa0c55ad015", "e3b0c44298fc1c149afbf4c8996fb924"]}}
	image_digest: {type: "choice", params: {values: ["sha256:4f3c2a1b8d7e6f50918273645abcdeff", "sha256:9a0b1c2d3e4f567890abcdef12345678"]}}
	resource_uuid: {type: "uuid", params: {}}
	task_id: {
		type: "sequence"
		params: {prefix: "task-", start: 1000, step: 1, pad: 6}
	}
}

context: {
	global: {}
	system: credential_app: {
		service: {source: {const: "凭据应用"}}
	}
	instance: secret_exposure_flow: {
		host: {source: {generator: "host_sequence"}}
		username: {source: {generator: "username"}}
		user_password: {source: {generator: "human_password"}}
		api_key: {source: {generator: "api_key"}}
		bearer_token: {source: {generator: "bearer_token"}}
		cloud_pair: {source: {generator: "cloud_pair"}}
		session_id: {source: {generator: "session_id"}}
	}
	event: {
		request_id: {source: {generator: "request_id"}}
		trace_id: {source: {generator: "trace_id"}}
		version: {source: {generator: "version"}}
		git_commit: {source: {generator: "git_commit"}}
		image_digest: {source: {generator: "image_digest"}}
		resource_uuid: {source: {generator: "resource_uuid"}}
		task_id: {source: {generator: "task_id"}}
	}
}

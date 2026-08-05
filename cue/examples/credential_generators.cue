package rewrite

ir_version: "0.2"

systems: credentials: {
	kind: "credential_audit"
	description: "Synthetic credential lifecycle events"
}

generators: {
	human_password: {
		type: "corpus_choice"
		params: {
			source: {builtin: "human-passwords-v1"}
			min_length: 4
			max_length: 128
			sampling: "uniform"
			deduplicate: true
		}
	}
	weak_password: {type: "choice", params: values: ["123456", "password", "admin", "qwerty"]}
	hex_secret: {type: "machine_secret", params: {profile: "hex", byte_length: 16}}
	base64_secret: {type: "machine_secret", params: {profile: "base64", byte_length: 24}}
	base64url_secret: {type: "machine_secret", params: {profile: "base64url", byte_length: 24}}
	uuid_secret: {type: "machine_secret", params: {profile: "uuid"}}
	api_key: {type: "machine_secret", params: {profile: "api_key", prefix: "sim_sk_", body_length: 20, encoding: "base64url"}}
	bearer_token: {type: "machine_secret", params: {profile: "bearer_token", prefix: "sim_bearer_", body_length: 24, encoding: "alphanumeric"}}
	jwt: {type: "machine_secret", params: {profile: "jwt_shape", issuer: "rewrite", subject: "demo", signature_length: 24}}
	pem: {type: "machine_secret", params: {profile: "pem_shape", label: "SIMULATED PRIVATE KEY", byte_length: 48, line_width: 32}}
	cloud_access_key: {type: "machine_secret", params: {profile: "cloud_access_key", prefix: "SIMAKIA", body_length: 16}}
	cloud_pair: {type: "machine_secret", params: {profile: "cloud_secret_pair", id_prefix: "SIMAKIA", id_body_length: 16, secret_length: 40}}
	connection: {type: "machine_secret", params: {profile: "connection_string", scheme: "postgresql", host: "db.invalid", port: 5432, database: "simdb", username: "sim_user", password_length: 24}}
	private_token: {type: "machine_secret", params: {profile: "private_token", prefix: "sim_tok_", body_length: 24, encoding: "hex"}}
}

context: {
	global: {
		global_hex: {source: {generator: "hex_secret"}}
	}
	system: credentials: {
		system_base64: {source: {generator: "base64_secret"}}
	}
	instance: credential_flow: {
		human_password: {source: {generator: "human_password"}}
		cloud_pair: {source: {generator: "cloud_pair"}}
		api_key: {source: {generator: "api_key"}}
	}
	event: {
		weak_password: {source: {generator: "weak_password"}}
		base64url_secret: {source: {generator: "base64url_secret"}}
		uuid_secret: {source: {generator: "uuid_secret"}}
		bearer_token: {source: {generator: "bearer_token"}}
		jwt: {source: {generator: "jwt"}}
		pem: {source: {generator: "pem"}}
		cloud_access_key: {source: {generator: "cloud_access_key"}}
		connection: {source: {generator: "connection"}}
		private_token: {source: {generator: "private_token"}}
	}
}

templates: credential_snapshot: {
	system: "credentials"
	event_type: "credential_snapshot"
	fields: {
		global_hex: {ref: "ctx.global.global_hex"}
		system_base64: {ref: "ctx.system.system_base64"}
		human_password: {ref: "ctx.instance.human_password"}
		api_key: {ref: "ctx.instance.api_key"}
		access_key_id: {ref: "ctx.instance.cloud_pair.access_key_id"}
		secret_access_key: {ref: "ctx.instance.cloud_pair.secret_access_key"}
		weak_password: {ref: "ctx.event.weak_password"}
		base64url_secret: {ref: "ctx.event.base64url_secret"}
		uuid_secret: {ref: "ctx.event.uuid_secret"}
		bearer_token: {ref: "ctx.event.bearer_token"}
		jwt: {ref: "ctx.event.jwt"}
		pem: {ref: "ctx.event.pem"}
		cloud_access_key: {ref: "ctx.event.cloud_access_key"}
		connection: {ref: "ctx.event.connection"}
		private_token: {ref: "ctx.event.private_token"}
	}
}

machines: credential_flow: {
	system: "credentials"
	start: "inventory"
	attack: {
		enabled: true
		type: "secret_exposure"
		technique: "T1552"
		tactic: "credential_access"
	}
	states: {
		inventory: {
			kind: "benign"
			emit: [{template: "credential_snapshot"}]
			transitions: [{to: "exposure", weight: 1}]
		}
		exposure: {
			kind: "attack"
			emit: [{template: "credential_snapshot"}]
			terminal: true
		}
	}
}

instance_groups: credential_instances: {
	machine: "credential_flow"
	enabled: true
	count: 2
}

time_profiles: default_time: {
	start: "2026-05-17T02:00:00Z"
	timezone: "UTC"
	granularity: "millisecond"
	monotonic_per_instance: true
	system_clock_skew: credentials: {milliseconds: 0}
}

scenario: {
	id: "credential_generators_demo"
	systems: ["credentials"]
	instance_groups: ["credential_instances"]
	time_profile: "default_time"
}

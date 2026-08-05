package rewrite

// Minimal end-to-end scenario for Phase 1.
// It exercises the MVP contract: one linux_auth system, one reusable SSH login
// machine, benign and attack branches, state-level attack-label override,
// emit.count range, context references and template reuse.

ir_version: "0.2"

systems: {
	linux_auth: {
		kind:        "linux_auth"
		description: "Linux authentication log domain for SSH login events"
	}
}

generators: {
	username_pool: {
		type: "choice"
		params: {
			values: ["alice", "bob", "carol", "dave", "ops"]
		}
		description: "Choose a target user for one SSH session"
	}

	attacker_ip_pool: {
		type: "ipv4"
		params: {
			cidr: "203.0.113.0/24"
		}
		description: "Source IP for the brute-force branch"
	}

	normal_ip_pool: {
		type: "ipv4"
		params: {
			cidr: "10.10.0.0/16"
		}
		description: "Source IP for ordinary user activity"
	}

	session_id: {
		type:        "uuid"
		description: "Stable session id inside one instance"
	}

	process_id: {
		type: "number_range"
		params: {
			min: 1000
			max: 65000
		}
		description: "Per-event process id"
	}

	password_candidate: {
		type: "choice"
		params: {
			values: ["password", "123456", "admin", "letmein", "Spring2026!"]
		}
		description: "One password tried in a brute-force event"
	}

	command_pool: {
		type: "choice"
		params: {
			values: ["whoami", "id", "pwd", "ls -la", "cat /etc/hostname"]
		}
		description: "Ordinary shell command after a successful login"
	}
}

context: {
	global: {
		tenant_id: {
			source: {const: "tenant-acme"}
			description: "Scenario-wide tenant identifier"
		}
		environment: {
			source: {const: "lab"}
		}
	}

	system: {
		linux_auth: {
			hostname: {
				source: {const: "ssh-gateway-01"}
			}
			facility: {
				source: {const: "auth"}
			}
			program: {
				source: {const: "sshd"}
			}
		}
	}

	instance: {
		ssh_login_flow: {
			user: {
				source: {generator: "username_pool"}
				description: "Stable user for one machine instance"
			}
			normal_src_ip: {
				source: {generator: "normal_ip_pool"}
			}
			attacker_ip: {
				source: {generator: "attacker_ip_pool"}
				description: "Stable attacker IP across brute-force and follow-up states"
			}
			session_id: {
				source: {generator: "session_id"}
			}
		}
	}

	event: {
		pid: {
			source: {generator: "process_id"}
		}
		password_candidate: {
			source: {generator: "password_candidate"}
		}
		shell_command: {
			source: {generator: "command_pool"}
		}
	}
}

templates: {
	ssh_login_success: {
		system:      "linux_auth"
		event_type:  "ssh_login_success"
		description: "Successful SSH authentication; reusable by benign and attack states"
		fields: {
			tenant_id: {ref: "ctx.global.tenant_id"}
			environment: {ref: "ctx.global.environment"}
			host: {ref: "ctx.system.hostname"}
			facility: {ref: "ctx.system.facility"}
			program: {ref: "ctx.system.program"}
			pid: {ref: "ctx.event.pid"}
			user: {ref: "ctx.instance.user"}
			session_id: {ref: "ctx.instance.session_id"}
			result: {const: "success"}
			message: {interpolate: "Accepted password for ${ctx.instance.user} from ${ctx.instance.normal_src_ip}"}
		}
	}

	ssh_login_failed: {
		system:      "linux_auth"
		event_type:  "ssh_login_failed"
		description: "Failed SSH authentication; attack meaning comes from state.kind, not template name"
		fields: {
			tenant_id: {ref: "ctx.global.tenant_id"}
			host: {ref: "ctx.system.hostname"}
			facility: {ref: "ctx.system.facility"}
			program: {ref: "ctx.system.program"}
			pid: {ref: "ctx.event.pid"}
			user: {ref: "ctx.instance.user"}
			src_ip: {ref: "ctx.instance.attacker_ip"}
			password_candidate: {ref: "ctx.event.password_candidate"}
			result: {const: "failed"}
			message: {interpolate: "Failed password for ${ctx.instance.user} from ${ctx.instance.attacker_ip}"}
		}
	}

	shell_command: {
		system:     "linux_auth"
		event_type: "shell_command"
		fields: {
			tenant_id: {ref: "ctx.global.tenant_id"}
			host: {ref: "ctx.system.hostname"}
			user: {ref: "ctx.instance.user"}
			session_id: {ref: "ctx.instance.session_id"}
			command: {ref: "ctx.event.shell_command"}
			result: {const: "executed"}
			message: {interpolate: "session ${ctx.instance.session_id}: ${ctx.instance.user} executed a shell command"}
		}
	}

	sudo_attempt: {
		system:     "linux_auth"
		event_type: "sudo_attempt"
		fields: {
			tenant_id: {ref: "ctx.global.tenant_id"}
			host: {ref: "ctx.system.hostname"}
			program: {const: "sudo"}
			user: {ref: "ctx.instance.user"}
			src_ip: {ref: "ctx.instance.attacker_ip"}
			session_id: {ref: "ctx.instance.session_id"}
			result: {const: "denied"}
			message: {interpolate: "sudo authentication failure for ${ctx.instance.user} from ${ctx.instance.attacker_ip}"}
		}
	}
}

machines: {
	ssh_login_flow: {
		system: "linux_auth"
		start:  "start"

		attack: {
			enabled:   true
			type:      "ssh_bruteforce"
			technique: "T1110"
			tactic:    "credential_access"
			description: "Default label for brute-force states inside this machine"
		}

		states: {
			start: {
				kind: "benign"
				transitions: [
					{to: "normal_login", weight: 95},
					{to: "bruteforce", weight: 5},
				]
			}

			normal_login: {
				kind: "benign"
				emit: [
					{template: "ssh_login_success"},
				]
				transitions: [
					{to: "normal_activity", weight: 1},
				]
			}

			normal_activity: {
				kind: "benign"
				emit: [
					{template: "shell_command"},
				]
				terminal: true
			}

			bruteforce: {
				kind: "attack"
				attack_label: {
					type:      "ssh_bruteforce"
					technique: "T1110"
					name:      "SSH brute-force password guessing"
				}
				emit: [
					{
						template: "ssh_login_failed"
						count: {min: 20, max: 80}
					},
				]
				transitions: [
					{to: "attack_success", weight: 1},
				]
			}

			attack_success: {
				kind: "attack"
				attack_label: {
					type:      "valid_account_login"
					technique: "T1078"
					tactic:    "defense_evasion"
					name:      "Valid account login after brute force"
				}
				emit: [
					{template: "ssh_login_success"},
				]
				transitions: [
					{to: "suspicious_activity", weight: 1},
				]
			}

			suspicious_activity: {
				kind: "attack"
				emit: [
					{
						template: "sudo_attempt"
						count: {min: 1, max: 5}
					},
				]
				terminal: true
			}
		}
	}
}

instance_groups: {
	ssh_login_sessions: {
		machine:     "ssh_login_flow"
		enabled:     true
		count:       20
		description: "Twenty independent SSH login-flow instances"
	}
}

time_profiles: {
	default_time: {
		start:                  "2026-05-17T00:00:00Z"
		timezone:               "UTC"
		granularity:            "millisecond"
		monotonic_per_instance: true
		system_clock_skew: {
			linux_auth: {milliseconds: 0}
		}
		event_time: {
			default: {
				base_delay_ms: {min_milliseconds: 800, max_milliseconds: 2500}
				behavior_jitter_ms: {min_milliseconds: -200, max_milliseconds: 500}
				error_jitter_ms: {min_milliseconds: -20, max_milliseconds: 20}
			}
			event_types: {
				ssh_login_success: {
					base_delay_ms: {min_milliseconds: 500, max_milliseconds: 1500}
					behavior_jitter_ms: {min_milliseconds: -100, max_milliseconds: 300}
					error_jitter_ms: {min_milliseconds: -20, max_milliseconds: 20}
				}
				ssh_login_failed: {
					base_delay_ms: {min_milliseconds: 700, max_milliseconds: 2200}
					behavior_jitter_ms: {min_milliseconds: -200, max_milliseconds: 500}
					error_jitter_ms: {min_milliseconds: -20, max_milliseconds: 20}
				}
			}
			states: {
				ssh_login_flow: {
					bruteforce: {
						base_delay_ms: {min_milliseconds: 20, max_milliseconds: 80}
						behavior_jitter_ms: {min_milliseconds: -10, max_milliseconds: 30}
						error_jitter_ms: {min_milliseconds: -2, max_milliseconds: 2}
					}
					attack_success: {
						base_delay_ms: {min_milliseconds: 800, max_milliseconds: 3000}
						behavior_jitter_ms: {min_milliseconds: -200, max_milliseconds: 1000}
						error_jitter_ms: {min_milliseconds: -20, max_milliseconds: 20}
					}
				}
			}
		}
	}
}

scenario: {
	id:              "ssh_bruteforce_demo"
	description:     "MVP demo: benign SSH login path and brute-force attack path in a single machine"
	systems:         ["linux_auth"]
	instance_groups: ["ssh_login_sessions"]
	time_profile:    "default_time"
}

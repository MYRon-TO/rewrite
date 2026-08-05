package rewrite

// Multi-instance sample for Phase 6.2.
// It demonstrates that several machine instances share global/system context
// while each instance keeps its own instance-scoped variables.

ir_version: "0.2"

systems: {
	linux_auth: {
		kind:        "linux_auth"
		description: "Linux authentication logs for independent SSH login sessions"
	}
}

generators: {
	username_pool: {
		type: "choice"
		params: {
			values: ["alice", "bob", "carol", "dave"]
		}
		description: "Per-instance login user"
	}

	source_ip_pool: {
		type: "ipv4"
		params: {
			cidr: "10.20.0.0/16"
		}
		description: "Per-instance source IP"
	}

	session_id: {
		type:        "uuid"
		description: "Per-instance stable SSH session identifier"
	}

	process_id: {
		type: "number_range"
		params: {
			min: 2000
			max: 65000
		}
		description: "Per-event process id"
	}
}

context: {
	global: {
		tenant_id: {
			source: {const: "tenant-shared"}
			description: "Shared by every instance in the scenario"
		}
		environment: {
			source: {const: "test"}
		}
	}

	system: {
		linux_auth: {
			hostname: {
				source: {const: "ssh-shared-01"}
				description: "Shared by every instance for linux_auth"
			}
			program: {
				source: {const: "sshd"}
			}
		}
	}

	instance: {
		ssh_login_once: {
			user: {
				source: {generator: "username_pool"}
				description: "Stable only inside one instance"
			}
			src_ip: {
				source: {generator: "source_ip_pool"}
			}
			session_id: {
				source: {generator: "session_id"}
				description: "Used by tests to prove instance variables are isolated"
			}
		}
	}

	event: {
		pid: {
			source: {generator: "process_id"}
		}
	}
}

templates: {
	ssh_login_success: {
		system:      "linux_auth"
		event_type:  "ssh_login_success"
		description: "One successful SSH login event per instance"
		fields: {
			tenant_id: {ref: "ctx.global.tenant_id"}
			environment: {ref: "ctx.global.environment"}
			host: {ref: "ctx.system.hostname"}
			program: {ref: "ctx.system.program"}
			pid: {ref: "ctx.event.pid"}
			user: {ref: "ctx.instance.user"}
			src_ip: {ref: "ctx.instance.src_ip"}
			session_id: {ref: "ctx.instance.session_id"}
			result: {const: "success"}
			message: {interpolate: "Accepted password for ${ctx.instance.user} from ${ctx.instance.src_ip} on ${ctx.system.hostname}"}
		}
	}
}

machines: {
	ssh_login_once: {
		system: "linux_auth"
		start:  "login_success"
		attack: {
			enabled: false
		}
		states: {
			login_success: {
				kind: "benign"
				emit: [
					{template: "ssh_login_success"},
				]
				terminal: true
			}
		}
		description: "Single benign login event; repetition comes from instance_group.count"
	}
}

instance_groups: {
	ssh_login_sessions: {
		machine:     "ssh_login_once"
		enabled:     true
		count:       3
		description: "Three independent instances of the same login machine"
	}
}

time_profiles: {
	default_time: {
		start:                  "2026-05-17T01:00:00Z"
		timezone:               "UTC"
		granularity:            "millisecond"
		monotonic_per_instance: true
		system_clock_skew: {
			linux_auth: {milliseconds: 0}
		}
	}
}

scenario: {
	id:              "multi_instance_login_demo"
	description:     "MVP demo: multiple instances share global/system context but isolate instance context"
	systems:         ["linux_auth"]
	instance_groups: ["ssh_login_sessions"]
	time_profile:    "default_time"
}

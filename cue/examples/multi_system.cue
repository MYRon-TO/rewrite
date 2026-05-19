package rewrite

// Multi-system sample for Phase 6.3.
// It demonstrates that different systems can participate in one scenario while
// sharing global context and avoiding cross-system dependencies or causality.

ir_version: "0.1"

systems: {
	linux_auth: {
		kind:        "linux_auth"
		description: "Linux SSH authentication logs"
	}
	web_access: {
		kind:        "web_access"
		description: "HTTP access logs from an application edge"
	}
}

generators: {
	username_pool: {
		type: "choice"
		params: {
			values: ["alice", "bob", "carol", "dave"]
		}
	}

	client_ip_pool: {
		type: "ipv4"
		params: {
			cidr: "198.51.100.0/24"
		}
	}

	request_id: {
		type: "uuid"
	}

	process_id: {
		type: "number_range"
		params: {
			min: 1000
			max: 65000
		}
	}

	path_pool: {
		type: "choice"
		params: {
			values: ["/", "/login", "/api/status", "/docs"]
		}
	}

	status_pool: {
		type: "choice"
		params: {
			values: [200, 200, 200, 302, 404]
		}
	}
}

context: {
	global: {
		tenant_id: {
			source: {const: "tenant-multi-system"}
			description: "Shared by every participating system"
		}
		environment: {
			source: {const: "integration"}
		}
	}

	system: {
		linux_auth: {
			hostname: {
				source: {const: "ssh-edge-01"}
			}
			service: {
				source: {const: "sshd"}
			}
		}
		web_access: {
			hostname: {
				source: {const: "web-edge-01"}
			}
			service: {
				source: {const: "nginx"}
			}
		}
	}

	instance: {
		ssh_login_once: {
			user: {
				source: {generator: "username_pool"}
			}
			src_ip: {
				source: {generator: "client_ip_pool"}
			}
			session_id: {
				source: {generator: "request_id"}
			}
		}
		web_request_once: {
			client_ip: {
				source: {generator: "client_ip_pool"}
			}
			request_id: {
				source: {generator: "request_id"}
			}
			path: {
				source: {generator: "path_pool"}
			}
		}
	}

	event: {
		pid: {
			source: {generator: "process_id"}
		}
		status: {
			source: {generator: "status_pool"}
		}
	}
}

templates: {
	ssh_login_success: {
		system:      "linux_auth"
		event_type:  "ssh_login_success"
		description: "Linux auth event generated independently from web events"
		fields: {
			tenant_id: {ref: "ctx.global.tenant_id"}
			environment: {ref: "ctx.global.environment"}
			host: {ref: "ctx.system.hostname"}
			service: {ref: "ctx.system.service"}
			pid: {ref: "ctx.event.pid"}
			user: {ref: "ctx.instance.user"}
			src_ip: {ref: "ctx.instance.src_ip"}
			session_id: {ref: "ctx.instance.session_id"}
			result: {const: "success"}
			message: {interpolate: "Accepted password for ${ctx.instance.user} from ${ctx.instance.src_ip} on ${ctx.system.hostname}"}
		}
	}

	http_access: {
		system:      "web_access"
		event_type:  "http_access"
		description: "Web access event generated independently from auth events"
		fields: {
			tenant_id: {ref: "ctx.global.tenant_id"}
			environment: {ref: "ctx.global.environment"}
			host: {ref: "ctx.system.hostname"}
			service: {ref: "ctx.system.service"}
			client_ip: {ref: "ctx.instance.client_ip"}
			request_id: {ref: "ctx.instance.request_id"}
			path: {ref: "ctx.instance.path"}
			status: {ref: "ctx.event.status"}
			message: {interpolate: "${ctx.instance.client_ip} requested ${ctx.instance.path} on ${ctx.system.hostname}"}
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
	}

	web_request_once: {
		system: "web_access"
		start:  "access"
		attack: {
			enabled: false
		}
		states: {
			access: {
				kind: "benign"
				emit: [
					{template: "http_access"},
				]
				terminal: true
			}
		}
	}
}

instance_groups: {
	ssh_login_sessions: {
		machine:     "ssh_login_once"
		enabled:     true
		count:       2
		description: "Independent linux_auth instances"
	}
	web_requests: {
		machine:     "web_request_once"
		enabled:     true
		count:       2
		description: "Independent web_access instances"
	}
}

time_profiles: {
	default_time: {
		start:                  "2026-05-17T02:00:00Z"
		timezone:               "UTC"
		granularity:            "millisecond"
		monotonic_per_instance: true
		system_clock_skew: {
			linux_auth: {milliseconds: 0}
			web_access: {milliseconds: 250}
		}
	}
}

scenario: {
	id:              "multi_system_demo"
	description:     "MVP demo: multiple systems share global context without cross-system dependencies"
	systems:         ["linux_auth", "web_access"]
	instance_groups: ["ssh_login_sessions", "web_requests"]
	time_profile:    "default_time"
}

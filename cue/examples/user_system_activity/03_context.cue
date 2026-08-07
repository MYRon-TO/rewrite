package rewrite

// Context bindings: how systems, FSM instances and events source their values.

context: {
	global: {
		environment: {
			source: {const: "demo"}
		}
	}
	system: {
		app_platform: {
			host: {
				source: {generator: "app_platform_host_sequence"}
			}
			service: {
				source: {const: "application"}
			}
		}
		system_service: {
			service: {
				source: {const: "systemd"}
			}
		}
	}
	instance: {
		user_activity_flow: {
			host: {
				source: {generator: "user_host_sequence"}
				description: "Stable autoincremented host for one user FSM instance"
			}
			actor: {
				source: {generator: "user_pool"}
				description: "Stable user name for one user FSM instance"
			}
			new_user: {
				source: {generator: "new_user_pool"}
			}
			target_user: {
				source: {generator: "target_user_pool"}
			}
			file: {
				source: {generator: "file_pool"}
			}
			table: {
				source: {generator: "table_pool"}
			}
			session: {
				source: {generator: "session_id"}
			}
		}
		system_health_flow: {
			host: {
				source: {generator: "system_host_sequence"}
				description: "Stable autoincremented host for one system FSM instance"
			}
			actor: {
				source: {const: "system"}
			}
		}
	}
	event: {
		cpu: {
			source: {generator: "cpu_percent"}
		}
		memory: {
			source: {generator: "memory_percent"}
		}
		available_memory: {
			source: {generator: "available_memory_gb"}
		}
		disk: {
			source: {generator: "disk_percent"}
		}
		available_disk: {
			source: {generator: "available_disk_gb"}
		}
		latency: {
			source: {generator: "network_latency_ms"}
		}
		bandwidth: {
			source: {generator: "bandwidth_percent"}
		}
	}
}

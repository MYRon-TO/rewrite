package rewrite

// System health FSM: startup and recurring monitoring.

machines: {
	system_health_flow: {
		system: "system_service"
		start:  "system_start"
		attack: {
			enabled: false
		}
		states: {
			system_start: {
				kind: "benign"
				emit: [{template: "system_start"}]
				transitions: [{to: "system_started", weight: 1}]
			}
			system_started: {
				kind: "benign"
				emit: [{template: "system_load_config"}, {template: "system_init_db_pool"}, {template: "system_start_service"}, {template: "system_started"}]
				transitions: [{to: "monitor_choice", weight: 1}]
			}
			monitor_choice: {
				kind: "benign"
				transitions: [
					{to: "security_monitor", weight: 25},
					{to: "app_monitor", weight: 25},
					{to: "resource_monitor", weight: 35},
					{to: "system_stop", weight: 15},
				]
			}
			security_monitor: {
				kind: "benign"
				emit: [{template: "security_monitor"}]
				transitions: [{to: "monitor_choice", weight: 1}]
			}
			app_monitor: {
				kind: "benign"
				emit: [{template: "app_monitor"}]
				transitions: [{to: "monitor_choice", weight: 1}]
			}
			resource_monitor: {
				kind: "benign"
				emit: [{template: "cpu_monitor"}, {template: "memory_monitor"}, {template: "disk_monitor"}, {template: "network_monitor"}]
				transitions: [{to: "monitor_choice", weight: 1}]
			}
			system_stop: {
				kind:     "benign"
				terminal: true
			}
		}
		description: "One system-level FSM for startup and recurring health monitoring."
	}
}

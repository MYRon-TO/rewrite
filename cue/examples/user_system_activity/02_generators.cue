package rewrite

// Generator pool definitions for users, hosts, files, tables and resource metrics.

generators: {
	user_pool: {
		type: "username"
		params: {}
	}
	new_user_pool: {
		type: "username"
		params: {}
	}
	target_user_pool: {
		type: "username"
		params: {}
	}
	file_pool: {
		type: "simple_filename"
		params: {}
	}
	table_pool: {
		type: "simple_words"
		params: {}
	}
	session_id: {
		type: "uuid"
	}
	user_host_sequence: {
		type: "sequence"
		params: {
			prefix: "user-host-"
			start:  1
			step:   1
			pad:    2
		}
		description: "Autoincremented host per user instance"
	}
	app_platform_host_sequence: {
		type: "sequence"
		params: {
			prefix: "app-host-"
			start:  1
			step:   1
			pad:    2
		}
		description: "Autoincremented host for app_platform system context"
	}
	system_host_sequence: {
		type: "sequence"
		params: {
			prefix: "system-host-"
			start:  1
			step:   1
			pad:    2
		}
		description: "Autoincremented host for system_service system context"
	}
	cpu_percent: {
		type: "number_range"
		params: {
			min: 5
			max: 95
		}
	}
	memory_percent: {
		type: "number_range"
		params: {
			min: 20
			max: 90
		}
	}
	available_memory_gb: {
		type: "number_range"
		params: {
			min: 2
			max: 64
		}
	}
	disk_percent: {
		type: "number_range"
		params: {
			min: 10
			max: 92
		}
	}
	available_disk_gb: {
		type: "number_range"
		params: {
			min: 20
			max: 2048
		}
	}
	network_latency_ms: {
		type: "number_range"
		params: {
			min: 1
			max: 250
		}
	}
	bandwidth_percent: {
		type: "number_range"
		params: {
			min: 1
			max: 80
		}
	}
}

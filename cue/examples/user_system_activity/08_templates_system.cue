package rewrite

// System lifecycle and monitoring event templates.

templates: {
	system_start: {
		system:     "system_service"
		event_type: "system_lifecycle"
		fields: {
			content: {const: "系统启动初始化"}
			host: {ref: "ctx.instance.host"}
		}
	}
	system_load_config: {
		system:     "system_service"
		event_type: "system_lifecycle"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "加载系统配置"}
		}
	}
	system_init_db_pool: {
		system:     "system_service"
		event_type: "system_lifecycle"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "初始化数据库连接池"}
		}
	}
	system_start_service: {
		system:     "system_service"
		event_type: "system_lifecycle"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "启动应用服务"}
		}
	}
	system_started: {
		system:     "system_service"
		event_type: "system_lifecycle"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "系统启动完成，等待用户访问"}
		}
	}
	security_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "系统安全监控服务启动，安全监控完成，未发现异常"}
			result: {const: "normal"}
		}
	}
	app_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "应用程序监控服务启动，应用程序监控完成，所有服务运行正常"}
			result: {const: "normal"}
		}
	}
	cpu_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "当前CPU使用率: ${ctx.event.cpu}%"}
		}
	}
	memory_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "当前内存使用率: ${ctx.event.memory}%，可用内存: ${ctx.event.available_memory}GB"}
		}
	}
	disk_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "磁盘使用率: ${ctx.event.disk}%，可用空间: ${ctx.event.available_disk}GB"}
		}
	}
	network_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "网络延迟: ${ctx.event.latency}ms，带宽使用率: ${ctx.event.bandwidth}%"}
		}
	}
}

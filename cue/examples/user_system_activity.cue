package rewrite

// Benign user/system activity scenario generated from task.md.
// Notes:
// - Multiple users are modeled as multiple instances of the same user_activity_flow FSM.
// - Login is the mandatory start path; user behavior states are reachable only after login_success.
// - Each login attempt can fail, retry, or abort; at most three attempts are modeled.
// - Attack behavior is intentionally disabled.

ir_version: "0.1"

systems: {
	app_platform: {
		kind:        "application_platform"
		description: "Application system that receives user operations"
	}
	system_service: {
		kind:        "system_service"
		description: "Background system services and health monitoring"
	}
}

generators: {
	user_pool: {
		type: "choice"
		params: {
			values: ["alice", "bob", "carol", "dave", "erin"]
		}
	}
	new_user_pool: {
		type: "choice"
		params: {
			values: ["newton", "mallory", "oscar", "peggy"]
		}
	}
	target_user_pool: {
		type: "choice"
		params: {
			values: ["guest01", "contractor02", "temp03"]
		}
	}
	file_pool: {
		type: "choice"
		params: {
			values: ["report.pdf", "invoice.xlsx", "avatar.png", "dataset.csv"]
		}
	}
	table_pool: {
		type: "choice"
		params: {
			values: ["users", "orders", "audit_logs", "files"]
		}
	}
	session_id: {
		type: "uuid"
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

context: {
	global: {
		log_time: {
			source: {const: "2025/11/19 4:00"}
			description: "Text log header timestamp required by task.md. Runtime timestamp is still emitted separately."
		}
		environment: {
			source: {const: "demo"}
		}
	}
	system: {
		app_platform: {
			host: {
				source: {const: "app-01"}
			}
			service: {
				source: {const: "application"}
			}
		}
		system_service: {
			host: {
				source: {const: "ops-01"}
			}
			service: {
				source: {const: "systemd"}
			}
		}
	}
	instance: {
		user_activity_flow: {
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

templates: {
	login_request: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}请求登录系统"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}请求登录系统"}
			host: {ref: "ctx.system.host"}
			service: {ref: "ctx.system.service"}
			user: {ref: "ctx.instance.actor"}
		}
	}
	login_verify: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			content: {interpolate: "验证用户${ctx.instance.actor}的凭据"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},验证用户${ctx.instance.actor}的凭据"}
			host: {ref: "ctx.system.host"}
			user: {ref: "ctx.instance.actor"}
		}
	}
	login_success: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			content: {interpolate: "为用户 ${ctx.instance.actor} 创建会话"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},为用户 ${ctx.instance.actor} 创建会话"}
			user: {ref: "ctx.instance.actor"}
			session: {ref: "ctx.instance.session"}
			result: {const: "success"}
		}
	}
	login_bad_credentials: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			content: {const: "用户名或密码错误"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户名或密码错误"}
			user: {ref: "ctx.instance.actor"}
			result: {const: "failure"}
		}
	}
	login_failure_record: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}登录失败，记录失败尝试"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}登录失败，记录失败尝试"}
			user: {ref: "ctx.instance.actor"}
			result: {const: "failure"}
		}
	}
	user_abort: {
		system:     "app_platform"
		event_type: "user_abort"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}中止并退出"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}中止并退出"}
			user: {ref: "ctx.instance.actor"}
		}
	}
	create_user_request: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}发起创建用户请求，新用户: ${ctx.instance.new_user}"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}发起创建用户请求，新用户: ${ctx.instance.new_user}"}
			user: {ref: "ctx.instance.actor"}
			new_user: {ref: "ctx.instance.new_user"}
		}
	}
	create_user_success: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			content: {interpolate: "验证用户${ctx.instance.actor}的用户管理权限；检查新用户${ctx.instance.new_user}是否已存在；创建新用户${ctx.instance.new_user}账户；设置新用户${ctx.instance.new_user}的初始权限；用户${ctx.instance.new_user}创建成功"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},验证用户${ctx.instance.actor}的用户管理权限；检查新用户${ctx.instance.new_user}是否已存在；创建新用户${ctx.instance.new_user}账户；设置新用户${ctx.instance.new_user}的初始权限；用户${ctx.instance.new_user}创建成功"}
			result: {const: "success"}
		}
	}
	create_user_failure: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			content: {interpolate: "验证用户${ctx.instance.actor}的用户管理权限失败，用户${ctx.instance.new_user}创建失败"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},验证用户${ctx.instance.actor}的用户管理权限失败，用户${ctx.instance.new_user}创建失败"}
			result: {const: "failure"}
		}
	}
	delete_user_success: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}发起删除用户请求，目标用户: ${ctx.instance.target_user}；验证用户${ctx.instance.actor}的用户管理权限；检查目标用户${ctx.instance.target_user}是否存在；删除用户${ctx.instance.target_user}账户；用户${ctx.instance.target_user}删除成功"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}发起删除用户请求，目标用户: ${ctx.instance.target_user}；验证用户${ctx.instance.actor}的用户管理权限；检查目标用户${ctx.instance.target_user}是否存在；删除用户${ctx.instance.target_user}账户；用户${ctx.instance.target_user}删除成功"}
			result: {const: "success"}
		}
	}
	file_upload_request: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}选择上传文件${ctx.instance.file}"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}选择上传文件${ctx.instance.file}"}
			file: {ref: "ctx.instance.file"}
		}
	}
	file_upload_success: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			content: {interpolate: "验证文件${ctx.instance.file}格式和大小；准备文件${ctx.instance.file}传输；文件${ctx.instance.file}上传完成；更新文件${ctx.instance.file}索引"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},验证文件${ctx.instance.file}格式和大小；准备文件${ctx.instance.file}传输；文件${ctx.instance.file}上传完成；更新文件${ctx.instance.file}索引"}
			result: {const: "success"}
		}
	}
	file_upload_failure: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			content: {interpolate: "验证用户${ctx.instance.actor}对文件${ctx.instance.file}的访问权限；文件${ctx.instance.file}访问被拒绝；文件${ctx.instance.file}上传失败，返回错误信息"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},验证用户${ctx.instance.actor}对文件${ctx.instance.file}的访问权限；文件${ctx.instance.file}访问被拒绝；文件${ctx.instance.file}上传失败，返回错误信息"}
			result: {const: "failure"}
		}
	}
	file_download_success: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}请求下载文件${ctx.instance.file}；验证用户${ctx.instance.actor}对文件${ctx.instance.file}的访问权限；开始文件${ctx.instance.file}下载；文件${ctx.instance.file}下载完成，更新下载记录"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}请求下载文件${ctx.instance.file}；验证用户${ctx.instance.actor}对文件${ctx.instance.file}的访问权限；开始文件${ctx.instance.file}下载；文件${ctx.instance.file}下载完成，更新下载记录"}
			result: {const: "success"}
		}
	}
	db_query_request: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}发起数据库查询请求"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}发起数据库查询请求"}
			table: {ref: "ctx.instance.table"}
		}
	}
	db_query_success: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			content: {interpolate: "构建查询语句，目标表: ${ctx.instance.table}；执行对表 ${ctx.instance.table} 的查询操作；处理查询结果；返回表${ctx.instance.table}的查询数据"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},构建查询语句，目标表: ${ctx.instance.table}；执行对表 ${ctx.instance.table} 的查询操作；处理查询结果；返回表${ctx.instance.table}的查询数据"}
			result: {const: "success"}
		}
	}
	db_update_success: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			content: {interpolate: "用户${ctx.instance.actor}发起数据更新请求；开始事务处理，目标表: ${ctx.instance.table}；执行对表 ${ctx.instance.table} 的更新操作；验证表${ctx.instance.table} 数据完整性；提交事务；表${ctx.instance.table}更新完成，关闭连接"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},用户${ctx.instance.actor}发起数据更新请求；开始事务处理，目标表: ${ctx.instance.table}；执行对表 ${ctx.instance.table} 的更新操作；验证表${ctx.instance.table} 数据完整性；提交事务；表${ctx.instance.table}更新完成，关闭连接"}
			result: {const: "success"}
		}
	}
	db_failure: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			content: {interpolate: "数据库连接超时/查询语法错误，目标表: ${ctx.instance.table}；对表 ${ctx.instance.table} 的查询执行失败；返回表${ctx.instance.table}查询错误信息"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},数据库连接超时/查询语法错误，目标表: ${ctx.instance.table}；对表 ${ctx.instance.table} 的查询执行失败；返回表${ctx.instance.table}查询错误信息"}
			result: {const: "failure"}
		}
	}
	system_start: {
		system:     "system_service"
		event_type: "system_lifecycle"
		fields: {
			content: {const: "系统启动初始化"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},系统启动初始化"}
			host: {ref: "ctx.system.host"}
		}
	}
	system_started: {
		system:     "system_service"
		event_type: "system_lifecycle"
		fields: {
			content: {const: "加载系统配置；初始化数据库连接池；启动应用服务；系统启动完成，等待用户访问"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},加载系统配置；初始化数据库连接池；启动应用服务；系统启动完成，等待用户访问"}
		}
	}
	security_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			content: {const: "系统安全监控服务启动，安全监控完成，未发现异常"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},系统安全监控服务启动，安全监控完成，未发现异常"}
			result: {const: "normal"}
		}
	}
	app_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			content: {const: "应用程序监控服务启动，应用程序监控完成，所有服务运行正常"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},应用程序监控服务启动，应用程序监控完成，所有服务运行正常"}
			result: {const: "normal"}
		}
	}
	resource_monitor: {
		system:     "system_service"
		event_type: "system_monitor"
		fields: {
			content: {interpolate: "当前CPU使用率: ${ctx.event.cpu}%；当前内存使用率: ${ctx.event.memory}%，可用内存: ${ctx.event.available_memory}GB；磁盘使用率: ${ctx.event.disk}%，可用空间: ${ctx.event.available_disk}GB；网络延迟: ${ctx.event.latency}ms，带宽使用率: ${ctx.event.bandwidth}%"}
			log_line: {interpolate: "${ctx.global.log_time},${ctx.instance.actor},当前CPU使用率: ${ctx.event.cpu}%；当前内存使用率: ${ctx.event.memory}%，可用内存: ${ctx.event.available_memory}GB；磁盘使用率: ${ctx.event.disk}%，可用空间: ${ctx.event.available_disk}GB；网络延迟: ${ctx.event.latency}ms，带宽使用率: ${ctx.event.bandwidth}%"}
		}
	}
}

machines: {
	user_activity_flow: {
		system: "app_platform"
		start:  "login_attempt_1_request"
		attack: {
			enabled: false
		}
		states: {
			login_attempt_1_request: {
				kind: "benign"
				emit: [{template: "login_request"}]
				transitions: [{to: "login_attempt_1_verify", weight: 1}]
			}
			login_attempt_1_verify: {
				kind: "benign"
				emit: [{template: "login_verify"}]
				transitions: [
					{to: "login_success", weight: 70},
					{to: "login_attempt_1_failed", weight: 25},
					{to: "user_abort", weight: 5},
				]
			}
			login_attempt_1_failed: {
				kind: "benign"
				emit: [{template: "login_bad_credentials"}, {template: "login_failure_record"}]
				transitions: [
					{to: "login_attempt_2_request", weight: 80},
					{to: "user_abort", weight: 20},
				]
			}
			login_attempt_2_request: {
				kind: "benign"
				emit: [{template: "login_request"}]
				transitions: [{to: "login_attempt_2_verify", weight: 1}]
			}
			login_attempt_2_verify: {
				kind: "benign"
				emit: [{template: "login_verify"}]
				transitions: [
					{to: "login_success", weight: 75},
					{to: "login_attempt_2_failed", weight: 20},
					{to: "user_abort", weight: 5},
				]
			}
			login_attempt_2_failed: {
				kind: "benign"
				emit: [{template: "login_bad_credentials"}, {template: "login_failure_record"}]
				transitions: [
					{to: "login_attempt_3_request", weight: 75},
					{to: "user_abort", weight: 25},
				]
			}
			login_attempt_3_request: {
				kind: "benign"
				emit: [{template: "login_request"}]
				transitions: [{to: "login_attempt_3_verify", weight: 1}]
			}
			login_attempt_3_verify: {
				kind: "benign"
				emit: [{template: "login_verify"}]
				transitions: [
					{to: "login_success", weight: 80},
					{to: "login_attempt_3_failed", weight: 15},
					{to: "user_abort", weight: 5},
				]
			}
			login_attempt_3_failed: {
				kind: "benign"
				emit: [{template: "login_bad_credentials"}, {template: "login_failure_record"}]
				terminal: true
			}
			login_success: {
				kind: "benign"
				emit: [{template: "login_success"}]
				transitions: [
					{to: "choose_behavior", weight: 95},
					{to: "user_abort", weight: 5},
				]
			}
			choose_behavior: {
				kind: "benign"
				transitions: [
					{to: "create_user_request", weight: 18},
					{to: "delete_user", weight: 10},
					{to: "file_upload_request", weight: 20},
					{to: "file_download", weight: 12},
					{to: "db_query_request", weight: 20},
					{to: "db_update", weight: 10},
					{to: "user_abort", weight: 10},
				]
			}
			create_user_request: {
				kind: "benign"
				emit: [{template: "create_user_request"}]
				transitions: [
					{to: "create_user_success", weight: 75},
					{to: "create_user_failure", weight: 25},
				]
			}
			create_user_success: {
				kind: "benign"
				emit: [{template: "create_user_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			create_user_failure: {
				kind: "benign"
				emit: [{template: "create_user_failure"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			delete_user: {
				kind: "benign"
				emit: [{template: "delete_user_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			file_upload_request: {
				kind: "benign"
				emit: [{template: "file_upload_request"}]
				transitions: [
					{to: "file_upload_success", weight: 70},
					{to: "file_upload_failure", weight: 30},
				]
			}
			file_upload_success: {
				kind: "benign"
				emit: [{template: "file_upload_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			file_upload_failure: {
				kind: "benign"
				emit: [{template: "file_upload_failure"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			file_download: {
				kind: "benign"
				emit: [{template: "file_download_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			db_query_request: {
				kind: "benign"
				emit: [{template: "db_query_request"}]
				transitions: [
					{to: "db_query_success", weight: 80},
					{to: "db_failure", weight: 20},
				]
			}
			db_query_success: {
				kind: "benign"
				emit: [{template: "db_query_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			db_update: {
				kind: "benign"
				emit: [{template: "db_update_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			db_failure: {
				kind: "benign"
				emit: [{template: "db_failure"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			post_behavior: {
				kind: "benign"
				transitions: [
					{to: "choose_behavior", weight: 80},
					{to: "user_abort", weight: 20},
				]
			}
			user_abort: {
				kind: "benign"
				emit: [{template: "user_abort"}]
				terminal: true
			}
		}
		description: "Same-type user FSM. Starts with login and reaches non-login behavior only after login_success."
	}
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
				emit: [{template: "system_started"}]
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
				emit: [{template: "resource_monitor"}]
				transitions: [{to: "monitor_choice", weight: 1}]
			}
			system_stop: {
				kind: "benign"
				terminal: true
			}
		}
		description: "One system-level FSM for startup and recurring health monitoring."
	}
}

instance_groups: {
	users: {
		machine:     "user_activity_flow"
		enabled:     true
		count:       5
		description: "Five users running the same finite-state machine"
	}
	system: {
		machine:     "system_health_flow"
		enabled:     true
		count:       1
		description: "One background system FSM"
	}
}

time_profiles: {
	default_time: {
		start:                  "2025-11-19T04:00:00Z"
		timezone:               "UTC"
		granularity:            "second"
		monotonic_per_instance: true
		system_clock_skew: {
			app_platform: {milliseconds: 0}
			system_service: {milliseconds: 0}
		}
	}
}

scenario: {
	id:              "user_system_activity_demo"
	description:     "Benign multi-user activity plus one system FSM; login is mandatory before user actions."
	systems:         ["app_platform", "system_service"]
	instance_groups: ["users", "system"]
	time_profile:    "default_time"
}

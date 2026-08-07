package rewrite

// Ordinary logs, redacted placeholders, and deliberately difficult negatives.
templates: {
	login_request: {
		system:     "credential_app"
		event_type: "login"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "用户 ${ctx.instance.username} 请求登录"}}
	}
	login_verify: {
		system:     "credential_app"
		event_type: "login"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "开始验证用户 ${ctx.instance.username} 的密码"}}
	}
	login_success: {
		system:     "credential_app"
		event_type: "login"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "用户 ${ctx.instance.username} 登录成功"}}
	}
	task_complete: {
		system:     "credential_app"
		event_type: "task"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "任务 ${ctx.event.task_id} 执行完成"}}
	}
	api_success: {
		system:     "credential_app"
		event_type: "api_call"
		fields: {host: {ref: "ctx.instance.host"}, content: {const: "外部接口调用成功"}}
	}
	cloud_success: {
		system:     "credential_app"
		event_type: "cloud_call"
		fields: {host: {ref: "ctx.instance.host"}, content: {const: "云服务调用成功"}}
	}
	session_closed: {
		system:     "credential_app"
		event_type: "session"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "会话 ${ctx.instance.session_id} 已关闭"}}
	}
	password_placeholder: {
		system:     "credential_app"
		event_type: "login"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "用户 ${ctx.instance.username} 登录失败，密码=<your-password-here>"}}
	}
	secret_redacted: {
		system:     "credential_app"
		event_type: "cloud_call"
		fields: {host: {ref: "ctx.instance.host"}, content: {const: "云服务配置 secret=******"}}
	}
	password_expired: {
		system:     "credential_app"
		event_type: "login"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "用户 ${ctx.instance.username} 的密码已过期"}}
	}
	key_rotation: {
		system:     "credential_app"
		event_type: "maintenance"
		fields: {host: {ref: "ctx.instance.host"}, content: {const: "云密钥轮换完成"}}
	}
	token_cache_clear: {
		system:     "credential_app"
		event_type: "maintenance"
		fields: {host: {ref: "ctx.instance.host"}, content: {const: "Token 缓存已清理"}}
	}
	request_complete: {
		system:     "credential_app"
		event_type: "request"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "请求 ${ctx.event.request_id} 处理完成"}}
	}
	trace_success: {
		system:     "credential_app"
		event_type: "request"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "链路 ${ctx.event.trace_id} 调用成功"}}
	}
	version_release: {
		system:     "credential_app"
		event_type: "deployment"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "构建版本 ${ctx.event.version} 已发布"}}
	}
	commit_deployed: {
		system:     "credential_app"
		event_type: "deployment"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "提交 ${ctx.event.git_commit} 已部署"}}
	}
	image_synced: {
		system:     "credential_app"
		event_type: "deployment"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "镜像 ${ctx.event.image_digest} 已同步"}}
	}
	resource_created: {
		system:     "credential_app"
		event_type: "cloud_call"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "资源 ${ctx.event.resource_uuid} 创建成功"}}
	}
}

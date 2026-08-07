package rewrite

// Login, retry and abort event templates.

templates: {
	login_request: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			content: {interpolate: "用户 ${ctx.instance.actor} 请求登录系统"}
			host: {ref: "ctx.instance.host"}
			service: {ref: "ctx.system.service"}
			user: {ref: "ctx.instance.actor"}
		}
	}
	login_verify: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			content: {interpolate: "验证用户 ${ctx.instance.actor} 的凭据"}
			host: {ref: "ctx.instance.host"}
			user: {ref: "ctx.instance.actor"}
		}
	}
	login_success: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "为用户 ${ctx.instance.actor} 创建会话"}
			user: {ref: "ctx.instance.actor"}
			session: {ref: "ctx.instance.session"}
			result: {const: "success"}
		}
	}
	login_bad_credentials: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "用户名或密码错误"}
			user: {ref: "ctx.instance.actor"}
			result: {const: "failure"}
		}
	}
	login_failure_record: {
		system:     "app_platform"
		event_type: "user_login"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.actor} 登录失败，记录失败尝试"}
			user: {ref: "ctx.instance.actor"}
			result: {const: "failure"}
		}
	}
	user_abort: {
		system:     "app_platform"
		event_type: "user_abort"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.actor} 中止并退出"}
			user: {ref: "ctx.instance.actor"}
		}
	}
}

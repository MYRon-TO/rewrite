package rewrite

// User creation and deletion event templates.

templates: {
	create_user_request: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.actor} 发起创建用户请求，新用户: ${ctx.instance.new_user}"}
			user: {ref: "ctx.instance.actor"}
			new_user: {ref: "ctx.instance.new_user"}
		}
	}
	create_user_check_permission: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "验证用户 ${ctx.instance.actor} 的用户管理权限"}
		}
	}
	create_user_check_exists: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "检查新用户 ${ctx.instance.new_user} 是否已存在"}
		}
	}
	create_user_create_account: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "创建新用户 ${ctx.instance.new_user} 账户"}
		}
	}
	create_user_set_permission: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "设置新用户 ${ctx.instance.new_user} 的初始权限"}
		}
	}
	create_user_success: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.new_user} 创建成功"}
			result: {const: "success"}
		}
	}
	create_user_failure: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "验证用户 ${ctx.instance.actor} 的用户管理权限失败，用户 ${ctx.instance.new_user} 创建失败"}
			result: {const: "failure"}
		}
	}
	delete_user_request: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.actor} 发起删除用户请求，目标用户: ${ctx.instance.target_user}"}
		}
	}
	delete_user_check_permission: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "验证用户 ${ctx.instance.actor} 的用户管理权限"}
		}
	}
	delete_user_check_exists: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "检查目标用户 ${ctx.instance.target_user} 是否存在"}
		}
	}
	delete_user_delete_account: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "删除用户 ${ctx.instance.target_user} 账户"}
		}
	}
	delete_user_success: {
		system:     "app_platform"
		event_type: "user_management"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.target_user} 删除成功"}
			result: {const: "success"}
		}
	}
}

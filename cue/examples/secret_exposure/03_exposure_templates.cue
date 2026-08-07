package rewrite

// Templates remain label-free; attack states decide whether rendered values are exposures.
templates: {
	password_exposed: {
		system:     "credential_app"
		event_type: "login_error"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "用户 ${ctx.instance.username} 登录失败，密码=${ctx.instance.user_password}"}}
	}
	api_key_exposed: {
		system:     "credential_app"
		event_type: "api_error"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "外部接口调用失败，api_key=${ctx.instance.api_key}"}}
	}
	bearer_exposed: {
		system:     "credential_app"
		event_type: "auth_error"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "认证失败，Authorization=Bearer ${ctx.instance.bearer_token}"}}
	}
	cloud_secret_exposed: {
		system:     "credential_app"
		event_type: "cloud_error"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "云服务认证失败，secret_access_key=${ctx.instance.cloud_pair.secret_access_key}"}}
	}
	password_nested: {
		system:     "credential_app"
		event_type: "login_error"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "登录失败，详情={\"用户\":\"${ctx.instance.username}\",\"密码\":\"${ctx.instance.user_password}\",\"结果\":\"拒绝\"}"}}
	}
	api_key_nested: {
		system:     "credential_app"
		event_type: "api_error"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "接口调用失败，详情={\"接口\":\"外部服务\",\"api_key\":\"${ctx.instance.api_key}\",\"请求\":\"${ctx.event.request_id}\"}"}}
	}
	bearer_nested: {
		system:     "credential_app"
		event_type: "auth_error"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "认证失败，详情={\"类型\":\"Bearer\",\"token\":\"${ctx.instance.bearer_token}\",\"结果\":\"拒绝\"}"}}
	}
	cloud_pair_nested: {
		system:     "credential_app"
		event_type: "cloud_error"
		fields: {host: {ref: "ctx.instance.host"}, content: {interpolate: "云服务失败，凭据=${ctx.instance.cloud_pair}"}}
	}
}

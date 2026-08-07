package rewrite

// Database query and update event templates.

templates: {
	db_query_request: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.actor} 发起数据库查询请求"}
			table: {ref: "ctx.instance.table"}
		}
	}
	db_build_query: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "构建查询语句，目标表: ${ctx.instance.table}"}
		}
	}
	db_execute_query: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "执行对表 ${ctx.instance.table} 的查询操作"}
		}
	}
	db_process_result: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "处理查询结果"}
		}
	}
	db_query_success: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "返回表 ${ctx.instance.table} 的查询数据"}
			result: {const: "success"}
		}
	}
	db_update_request: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.actor} 发起数据更新请求"}
		}
	}
	db_begin_transaction: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "开始事务处理，目标表: ${ctx.instance.table}"}
		}
	}
	db_execute_update: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "执行对表 ${ctx.instance.table} 的更新操作"}
		}
	}
	db_validate_integrity: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "验证表 ${ctx.instance.table} 数据完整性"}
		}
	}
	db_commit: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {const: "提交事务"}
		}
	}
	db_update_success: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "表 ${ctx.instance.table} 更新完成，关闭连接"}
			result: {const: "success"}
		}
	}
	db_error_reason: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "数据库连接超时/查询语法错误，目标表: ${ctx.instance.table}"}
		}
	}
	db_failure: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "对表 ${ctx.instance.table} 的查询执行失败"}
			result: {const: "failure"}
		}
	}
	db_return_error: {
		system:     "app_platform"
		event_type: "database_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "返回表 ${ctx.instance.table} 查询错误信息"}
			result: {const: "failure"}
		}
	}
}

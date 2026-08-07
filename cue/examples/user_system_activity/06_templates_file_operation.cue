package rewrite

// File upload and download event templates.

templates: {
	file_upload_request: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.actor} 选择上传文件 ${ctx.instance.file}"}
			file: {ref: "ctx.instance.file"}
		}
	}
	file_check_format: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "验证文件 ${ctx.instance.file} 格式和大小"}
		}
	}
	file_prepare_transfer: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "准备文件 ${ctx.instance.file} 传输"}
		}
	}
	file_upload_success: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "文件 ${ctx.instance.file} 上传完成"}
			result: {const: "success"}
		}
	}
	file_update_index: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "更新文件 ${ctx.instance.file} 索引"}
		}
	}
	file_check_permission: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "验证用户 ${ctx.instance.actor} 对文件 ${ctx.instance.file} 的访问权限"}
		}
	}
	file_access_denied: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "文件 ${ctx.instance.file} 访问被拒绝"}
		}
	}
	file_upload_failure: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "文件 ${ctx.instance.file} 上传失败，返回错误信息"}
			result: {const: "failure"}
		}
	}
	file_download_request: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "用户 ${ctx.instance.actor} 请求下载文件 ${ctx.instance.file}"}
		}
	}
	file_download_check_permission: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "验证用户 ${ctx.instance.actor} 对文件 ${ctx.instance.file} 的访问权限"}
		}
	}
	file_download_start: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "开始文件 ${ctx.instance.file} 下载"}
		}
	}
	file_download_success: {
		system:     "app_platform"
		event_type: "file_operation"
		fields: {
			host: {ref: "ctx.instance.host"}
			content: {interpolate: "文件 ${ctx.instance.file} 下载完成，更新下载记录"}
			result: {const: "success"}
		}
	}
}

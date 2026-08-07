package rewrite

// User activity FSM: benign behavior plus embedded attack branches.

machines: {
	user_activity_flow: {
		system: "app_platform"
		start:  "entry_point"
		attack: {
			enabled:     true
			type:        "user_activity_attack"
			description: "Attack branches embedded in user activity flow"
		}
		states: {
			entry_point: {
				kind: "benign"
				transitions: [
					{to: "login_attempt_1_request", weight: 82},
					{to: "attack_bypass_login_operation", weight: 5},
					{to: "attack_multiple_login", weight: 13},
				]
			}
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
					{to: "create_user_request", weight: 17},
					{to: "delete_user", weight: 9},
					{to: "file_upload_request", weight: 18},
					{to: "file_download", weight: 10},
					{to: "db_query_request", weight: 18},
					{to: "db_update", weight: 9},
					{to: "attack_delete_audit_logs", weight: 3},
					{to: "attack_partially_delete_audit_logs", weight: 7},
					{to: "user_abort", weight: 9},
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
				emit: [{template: "create_user_check_permission"}, {template: "create_user_check_exists"}, {template: "create_user_create_account"}, {template: "create_user_set_permission"}, {template: "create_user_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			create_user_failure: {
				kind: "benign"
				emit: [{template: "create_user_failure"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			delete_user: {
				kind: "benign"
				emit: [{template: "delete_user_request"}, {template: "delete_user_check_permission"}, {template: "delete_user_check_exists"}, {template: "delete_user_delete_account"}, {template: "delete_user_success"}]
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
				emit: [{template: "file_check_format"}, {template: "file_prepare_transfer"}, {template: "file_upload_success"}, {template: "file_update_index"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			file_upload_failure: {
				kind: "benign"
				emit: [{template: "file_check_permission"}, {template: "file_access_denied"}, {template: "file_upload_failure"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			file_download: {
				kind: "benign"
				emit: [{template: "file_download_request"}, {template: "file_download_check_permission"}, {template: "file_download_start"}, {template: "file_download_success"}]
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
				emit: [{template: "db_build_query"}, {template: "db_execute_query"}, {template: "db_process_result"}, {template: "db_query_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			db_update: {
				kind: "benign"
				emit: [{template: "db_update_request"}, {template: "db_begin_transaction"}, {template: "db_execute_update"}, {template: "db_validate_integrity"}, {template: "db_commit"}, {template: "db_update_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			db_failure: {
				kind: "benign"
				emit: [{template: "db_error_reason"}, {template: "db_failure"}, {template: "db_return_error"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_bypass_login_operation: {
				kind: "attack"
				attack_label: {
					type:      "authentication_bypass"
					technique: "T1550"
					tactic:    "defense_evasion"
					name:      "Deleted normal-looking login evidence before bypassed operation"
				}
				emit: [{template: "login_request"}, {template: "login_verify"}, {template: "login_success"}]
				transitions: [{to: "choose_behavior", weight: 1}]
			}
			attack_multiple_login: {
				kind: "attack"
				attack_label: {
					type:      "multiple_login_attempts"
					technique: "T1110"
					tactic:    "credential_access"
					name:      "Many normal-looking login attempts"
				}
				emit: [{template: "login_request"}, {template: "login_verify"}, {template: "login_bad_credentials"}, {template: "login_failure_record"}]
				transitions: [
					{to: "attack_multiple_login", weight: 90},
					{to: "login_attempt_1_request", weight: 10},
				]
			}
			attack_delete_audit_logs: {
				kind: "attack"
				transitions: [
					{to: "attack_deleted_create_user_logs", weight: 15},
					{to: "attack_deleted_delete_user_logs", weight: 15},
					{to: "attack_deleted_file_upload_logs", weight: 20},
					{to: "attack_deleted_file_download_logs", weight: 15},
					{to: "attack_deleted_db_query_logs", weight: 20},
					{to: "attack_deleted_db_update_logs", weight: 15},
				]
			}
			attack_deleted_create_user_logs: {
				kind: "attack"
				attack_label: {
					type:      "log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Deleted normal-looking user creation logs"
				}
				emit: [{template: "create_user_request"}, {template: "create_user_check_permission"}, {template: "create_user_check_exists"}, {template: "create_user_create_account"}, {template: "create_user_set_permission"}, {template: "create_user_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_deleted_delete_user_logs: {
				kind: "attack"
				attack_label: {
					type:      "log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Deleted normal-looking user deletion logs"
				}
				emit: [{template: "delete_user_request"}, {template: "delete_user_check_permission"}, {template: "delete_user_check_exists"}, {template: "delete_user_delete_account"}, {template: "delete_user_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_deleted_file_upload_logs: {
				kind: "attack"
				attack_label: {
					type:      "log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Deleted normal-looking file upload logs"
				}
				emit: [{template: "file_upload_request"}, {template: "file_check_format"}, {template: "file_prepare_transfer"}, {template: "file_upload_success"}, {template: "file_update_index"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_deleted_file_download_logs: {
				kind: "attack"
				attack_label: {
					type:      "log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Deleted normal-looking file download logs"
				}
				emit: [{template: "file_download_request"}, {template: "file_download_check_permission"}, {template: "file_download_start"}, {template: "file_download_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_deleted_db_query_logs: {
				kind: "attack"
				attack_label: {
					type:      "log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Deleted normal-looking database query logs"
				}
				emit: [{template: "db_query_request"}, {template: "db_build_query"}, {template: "db_execute_query"}, {template: "db_process_result"}, {template: "db_query_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_deleted_db_update_logs: {
				kind: "attack"
				attack_label: {
					type:      "log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Deleted normal-looking database update logs"
				}
				emit: [{template: "db_update_request"}, {template: "db_begin_transaction"}, {template: "db_execute_update"}, {template: "db_validate_integrity"}, {template: "db_commit"}, {template: "db_update_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_partially_delete_audit_logs: {
				kind: "attack"
				transitions: [
					{to: "attack_partial_create_user_request", weight: 15},
					{to: "attack_partial_delete_user_request", weight: 15},
					{to: "attack_partial_file_upload_request", weight: 20},
					{to: "attack_partial_file_download_request", weight: 15},
					{to: "attack_partial_db_query_request", weight: 20},
					{to: "attack_partial_db_update_request", weight: 15},
				]
			}
			attack_partial_create_user_request: {
				kind: "benign"
				emit: [{template: "create_user_request"}]
				transitions: [{to: "attack_partial_create_user_deleted_check_permission", weight: 1}]
			}
			attack_partial_create_user_deleted_check_permission: {
				kind: "attack"
				attack_label: {
					type:      "partial:log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Partially removed user creation audit logs"
				}
				emit: [{template: "create_user_check_permission"}]
				transitions: [{to: "attack_partial_create_user_check_exists", weight: 1}]
			}
			attack_partial_create_user_check_exists: {
				kind: "benign"
				emit: [{template: "create_user_check_exists"}]
				transitions: [{to: "attack_partial_create_user_deleted_create_account", weight: 1}]
			}
			attack_partial_create_user_deleted_create_account: {
				kind: "attack"
				attack_label: {
					type:      "partial:log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Partially removed user creation audit logs"
				}
				emit: [{template: "create_user_create_account"}]
				transitions: [{to: "attack_partial_create_user_remaining", weight: 1}]
			}
			attack_partial_create_user_remaining: {
				kind: "benign"
				emit: [{template: "create_user_set_permission"}, {template: "create_user_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_partial_delete_user_request: {
				kind: "benign"
				emit: [{template: "delete_user_request"}]
				transitions: [{to: "attack_partial_delete_user_deleted_checks", weight: 1}]
			}
			attack_partial_delete_user_deleted_checks: {
				kind: "attack"
				attack_label: {
					type:      "partial:log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Partially removed user deletion audit logs"
				}
				emit: [{template: "delete_user_check_permission"}, {template: "delete_user_check_exists"}]
				transitions: [{to: "attack_partial_delete_user_remaining", weight: 1}]
			}
			attack_partial_delete_user_remaining: {
				kind: "benign"
				emit: [{template: "delete_user_delete_account"}, {template: "delete_user_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_partial_file_upload_request: {
				kind: "benign"
				emit: [{template: "file_upload_request"}]
				transitions: [{to: "attack_partial_file_upload_deleted_validation", weight: 1}]
			}
			attack_partial_file_upload_deleted_validation: {
				kind: "attack"
				attack_label: {
					type:      "partial:log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Partially removed file upload audit logs"
				}
				emit: [{template: "file_check_format"}, {template: "file_prepare_transfer"}]
				transitions: [{to: "attack_partial_file_upload_remaining", weight: 1}]
			}
			attack_partial_file_upload_remaining: {
				kind: "benign"
				emit: [{template: "file_upload_success"}, {template: "file_update_index"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_partial_file_download_request: {
				kind: "benign"
				emit: [{template: "file_download_request"}]
				transitions: [{to: "attack_partial_file_download_deleted_access", weight: 1}]
			}
			attack_partial_file_download_deleted_access: {
				kind: "attack"
				attack_label: {
					type:      "partial:log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Partially removed file download audit logs"
				}
				emit: [{template: "file_download_check_permission"}, {template: "file_download_start"}]
				transitions: [{to: "attack_partial_file_download_success", weight: 1}]
			}
			attack_partial_file_download_success: {
				kind: "benign"
				emit: [{template: "file_download_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_partial_db_query_request: {
				kind: "benign"
				emit: [{template: "db_query_request"}]
				transitions: [{to: "attack_partial_db_query_deleted_execution", weight: 1}]
			}
			attack_partial_db_query_deleted_execution: {
				kind: "attack"
				attack_label: {
					type:      "partial:log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Partially removed database query audit logs"
				}
				emit: [{template: "db_build_query"}, {template: "db_execute_query"}]
				transitions: [{to: "attack_partial_db_query_remaining", weight: 1}]
			}
			attack_partial_db_query_remaining: {
				kind: "benign"
				emit: [{template: "db_process_result"}, {template: "db_query_success"}]
				transitions: [{to: "post_behavior", weight: 1}]
			}
			attack_partial_db_update_request: {
				kind: "benign"
				emit: [{template: "db_update_request"}]
				transitions: [{to: "attack_partial_db_update_deleted_execution", weight: 1}]
			}
			attack_partial_db_update_deleted_execution: {
				kind: "attack"
				attack_label: {
					type:      "partial:log_tampering"
					technique: "T1070"
					tactic:    "defense_evasion"
					name:      "Partially removed database update audit logs"
				}
				emit: [{template: "db_begin_transaction"}, {template: "db_execute_update"}]
				transitions: [{to: "attack_partial_db_update_remaining", weight: 1}]
			}
			attack_partial_db_update_remaining: {
				kind: "benign"
				emit: [{template: "db_validate_integrity"}, {template: "db_commit"}, {template: "db_update_success"}]
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
		description: "Same-type user FSM. Normal behavior starts with login; attack branches can model login bypass, repeated login, complete audit-log tampering, and partial audit-log tampering."
	}
}

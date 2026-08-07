package rewrite

// Benign user/system activity scenario — entry point.
//
// This file is the scenario-level entry: it declares the instance groups, the
// time profile and the scenario, and references the definitions that live in
// the sibling directory cue/examples/user_system_activity/:
//
//	01_systems.cue                        systems
//	02_generators.cue                     generators
//	03_context.cue                        context
//	04_templates_login.cue                login / abort templates
//	05_templates_user_management.cue      create/delete user templates
//	06_templates_file_operation.cue       file upload/download templates
//	07_templates_database.cue             database templates
//	08_templates_system.cue               system lifecycle/monitor templates
//	09_machines_user_activity_flow.cue    user_activity_flow FSM
//	10_machines_system_health_flow.cue    system_health_flow FSM
//
// The validation/export scripts automatically unify every *.cue file in that
// directory with this entry, so the whole example still exports as a single IR.
//
// Notes:
// - Multiple users are modeled as multiple instances of the same user_activity_flow FSM.
// - Login is the mandatory start path; user behavior states are reachable only after login_success.
// - Each login attempt can fail, retry, or abort; at most three attempts are modeled.
// - Attack behavior is intentionally disabled.

ir_version: "0.2"

instance_groups: {
	users: {
		machine:     "user_activity_flow"
		enabled:     true
		count:       400
		description: "Five users running the same finite-state machine"
	}
	system: {
		machine:     "system_health_flow"
		enabled:     true
		count:       40
		description: "One background system FSM"
	}
}

time_profiles: {
	default_time: {
		start:                  "2025-11-19T04:00:00Z"
		timezone:               "UTC"
		granularity:            "millisecond"
		monotonic_per_instance: true
		system_clock_skew: {
			app_platform: {milliseconds: 0}
			system_service: {milliseconds: 0}
		}
		event_time: {
			default: {
				base_delay_ms: {min_milliseconds: 800, max_milliseconds: 2500}
				behavior_jitter_ms: {min_milliseconds: -250, max_milliseconds: 600}
				error_jitter_ms: {min_milliseconds: -25, max_milliseconds: 25}
			}
			event_types: {
				user_login: {
					base_delay_ms: {min_milliseconds: 500, max_milliseconds: 1800}
					behavior_jitter_ms: {min_milliseconds: -150, max_milliseconds: 500}
					error_jitter_ms: {min_milliseconds: -20, max_milliseconds: 20}
				}
				user_management: {
					base_delay_ms: {min_milliseconds: 1000, max_milliseconds: 4000}
					behavior_jitter_ms: {min_milliseconds: -300, max_milliseconds: 1200}
					error_jitter_ms: {min_milliseconds: -30, max_milliseconds: 30}
				}
				file_operation: {
					base_delay_ms: {min_milliseconds: 1500, max_milliseconds: 8000}
					behavior_jitter_ms: {min_milliseconds: -500, max_milliseconds: 2500}
					error_jitter_ms: {min_milliseconds: -40, max_milliseconds: 40}
				}
				system_health: {
					base_delay_ms: {min_milliseconds: 10000, max_milliseconds: 60000}
					behavior_jitter_ms: {min_milliseconds: -2000, max_milliseconds: 5000}
					error_jitter_ms: {min_milliseconds: -50, max_milliseconds: 50}
				}
			}
			states: {
				user_activity_flow: {
					attack_multiple_login: {
						base_delay_ms: {min_milliseconds: 20, max_milliseconds: 80}
						behavior_jitter_ms: {min_milliseconds: -10, max_milliseconds: 30}
						error_jitter_ms: {min_milliseconds: -2, max_milliseconds: 2}
					}
				}
			}
		}
	}
}

scenario: {
	id:          "user_system_activity_demo"
	description: "Benign multi-user activity plus one system FSM; login is mandatory before user actions."
	systems: ["app_platform", "system_service"]
	instance_groups: ["users", "system"]
	time_profile: "default_time"
}

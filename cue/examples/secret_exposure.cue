package rewrite

// Entry point for the synthetic secret-exposure detection scenario.
ir_version: "0.2"

instance_groups: secret_exposure_users: {
	machine: "secret_exposure_flow"
	enabled: true
	count:   500
}

time_profiles: default_time: {
	start:                  "2026-05-17T02:00:00Z"
	timezone:               "UTC"
	granularity:            "millisecond"
	monotonic_per_instance: true
	system_clock_skew: credential_app: {milliseconds: 0}
}
scenario: {
	id:          "secret_exposure_detection"
	description: "Synthetic benign, hard-negative, and secret-exposure logs"
	systems: ["credential_app"]
	instance_groups: ["secret_exposure_users"]
	time_profile: "default_time"
}

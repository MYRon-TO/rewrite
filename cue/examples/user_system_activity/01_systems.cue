package rewrite

// Systems referenced by the scenario.

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

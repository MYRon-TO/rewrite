package rewrite

// Scenario composes systems, instance groups and a time profile for one run.
// It must not become an attack-chain or cross-system causality orchestrator.

#Scenario: close({
	id: #Identifier

	systems: [#Identifier, ...#Identifier]
	instance_groups: [#Identifier, ...#Identifier]
	time_profile: #Identifier

	description?: string
})

#InstanceGroup: close({
	id:      #Identifier
	machine: #Identifier

	enabled: *true | bool
	count:   int & >=1

	// Optional declarative parameters for this group. They are plain data, not
	// scripts and not a second attack-ratio control plane.
	params?:      #MVPJSONObject
	description?: string
})

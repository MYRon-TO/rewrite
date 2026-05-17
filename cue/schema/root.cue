package rewrite

// Common schema primitives and the normalized IR root shape.
//
// The root remains intentionally small: CUE owns structure, defaults and object
// boundaries; Rust owns execution and cross-reference validation.

#Identifier: =~"^[A-Za-z][A-Za-z0-9_-]*$"
#FieldName:  =~"^[A-Za-z_][A-Za-z0-9_]*$"

#JSON: null | bool | number | string | [...#JSON] | {
	[string]: #JSON
}
#JSONObject: {
	[string]: #JSON
}

#ContextRef: =~"^ctx\\.(global|system|instance|event)\\.[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*$"

// Closed schema for the exported JSON IR. CUE source files are merged at the
// package root, so top-level closure is enforced by validating the exported JSON
// against this definition in scripts/export-ir.sh.
#IR: close({
	ir_version: *"0.1" | string

	systems?: [ID=#Identifier]: #System & {
		id: ID
	}

	generators?: [ID=#Identifier]: #Generator & {
		id: ID
	}

	context?: #Context

	templates?: [ID=#Identifier]: #Template & {
		id: ID
	}

	machines?: [ID=#Identifier]: #Machine & {
		id: ID
	}

	instance_groups?: [ID=#Identifier]: #InstanceGroup & {
		id: ID
	}

	time_profiles?: [ID=#Identifier]: #TimeProfile & {
		id: ID
	}

	scenario?: #Scenario
})

ir_version: *"0.1" | string

systems?: [ID=#Identifier]: #System & {
	id: ID
}

generators?: [ID=#Identifier]: #Generator & {
	id: ID
}

context?: #Context

templates?: [ID=#Identifier]: #Template & {
	id: ID
}

machines?: [ID=#Identifier]: #Machine & {
	id: ID
}

instance_groups?: [ID=#Identifier]: #InstanceGroup & {
	id: ID
}

time_profiles?: [ID=#Identifier]: #TimeProfile & {
	id: ID
}

scenario?: #Scenario

// Explicit MVP boundary guards for common drift fields in CUE source. The final
// exported JSON is additionally checked against the closed #IR schema.
entity?:       null & string
attack_chain?: null & string
attack_ratio?: null & string
stage?:        null & string

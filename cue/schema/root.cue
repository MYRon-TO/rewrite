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

// JSON-compatible parameter payload that still respects the MVP boundary.
// Literal template constants use #JSON instead, so business payloads can contain
// arbitrary log data while params cannot hide execution hooks or future-model fields.
#MVPJSON: null | bool | number | string | [...#MVPJSON] | #MVPJSONObject
#MVPJSONObject: {
	[string]: #MVPJSON

	entity?:                      _|_
	entities?:                    _|_
	attack_chain?:                _|_
	attack_chains?:               _|_
	attack_ratio?:                _|_
	attack_quota?:                _|_
	attack_rate?:                 _|_
	stage?:                       _|_
	stages?:                      _|_
	attack_stage?:                _|_
	attack_stages?:               _|_
	dag?:                         _|_
	event_dag?:                   _|_
	events_dag?:                  _|_
	causal_dag?:                  _|_
	causality?:                   _|_
	causal_links?:                _|_
	causal_edges?:                _|_
	depends_on?:                  _|_
	dependencies?:                _|_
	cross_system?:                _|_
	cross_system_dependency?:     _|_
	cross_system_dependencies?:   _|_
	system_dependency?:           _|_
	system_dependencies?:         _|_
	depends_on_system?:           _|_
	target_system?:               _|_
	source_system?:               _|_
	cross_instance?:              _|_
	cross_instance_dependency?:   _|_
	cross_instance_dependencies?: _|_
	instance_dependency?:         _|_
	instance_dependencies?:       _|_
	depends_on_instance?:         _|_
	target_instance?:             _|_
	source_instance?:             _|_
	script?:                      _|_
	scripts?:                     _|_
	eval?:                        _|_
	exec?:                        _|_
	shell?:                       _|_
	hook?:                        _|_
	hooks?:                       _|_
	plugin?:                      _|_
	plugins?:                     _|_
	external?:                    _|_
	external_generator?:          _|_
	external_generators?:         _|_
}

#ContextRef: =~"^ctx\\.(global|system|instance|event)\\.[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*$"

// Context references use the normalized IR form ctx.<scope>.<name>[.<field>...].
// Scope-specific variants are used by context.cue to prevent long-lived values
// from depending on shorter-lived values. For example, a global context value
// may not reference ctx.event.*.
#GlobalContextRef:   =~"^ctx\\.global\\.[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*$"
#SystemContextRef:   =~"^ctx\\.(global|system)\\.[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*$"
#InstanceContextRef: =~"^ctx\\.(global|system|instance)\\.[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*$"
#EventContextRef:    #ContextRef

// A template interpolation is intentionally tiny: plain text plus at least one
// ${ctx.<scope>.<name>} placeholder. This rejects ${...} placeholders that are
// not context references, so interpolation cannot quietly turn into a second
// expression language.
#InterpolationTemplate: string & =~"^([^$]|\\$[^\\{])*(\\$\\{ctx\\.(global|system|instance|event)\\.[A-Za-z_][A-Za-z0-9_]*(\\.[A-Za-z_][A-Za-z0-9_]*)*\\}([^$]|\\$[^\\{])*)+$"

// Shared source-expression variants used by context and template schemas.
#ConstValueSource: close({
	const: #JSON
})

#GeneratorValueSource: close({
	generator: #Identifier
	params: *{} | #MVPJSONObject
})

#ContextRefValueSource: close({
	ref: #ContextRef
})

#GlobalContextRefValueSource: close({
	ref: #GlobalContextRef
})

#SystemContextRefValueSource: close({
	ref: #SystemContextRef
})

#InstanceContextRefValueSource: close({
	ref: #InstanceContextRef
})

#EventContextRefValueSource: close({
	ref: #EventContextRef
})

#TemplateConstExpr: close({
	const: #JSON
})

#TemplateContextRefExpr: close({
	ref: #ContextRef
})

#TemplateInterpolationExpr: close({
	interpolate: #InterpolationTemplate
})

#TemplateValueExpr: #TemplateConstExpr | #TemplateContextRefExpr | #TemplateInterpolationExpr

#ContextValueSource:         #ConstValueSource | #GeneratorValueSource | #ContextRefValueSource
#GlobalContextValueSource:   #ConstValueSource | #GeneratorValueSource | #GlobalContextRefValueSource
#SystemContextValueSource:   #ConstValueSource | #GeneratorValueSource | #SystemContextRefValueSource
#InstanceContextValueSource: #ConstValueSource | #GeneratorValueSource | #InstanceContextRefValueSource
#EventContextValueSource:    #ConstValueSource | #GeneratorValueSource | #EventContextRefValueSource

#TemplateFieldName:   #FieldName & !="shared" & !="share" & !="scope" & !="cache" & !="script" & !="scripts" & !="eval" & !="exec" & !="shell" & !="hook" & !="hooks" & !="plugin" & !="plugins" & !="external" & !="external_generator" & !="external_generators" & !="entity" & !="entities" & !="attack_chain" & !="attack_chains" & !="attack_ratio" & !="attack_quota" & !="attack_rate" & !="stage" & !="stages" & !="attack_stage" & !="attack_stages" & !="dag" & !="event_dag" & !="events_dag" & !="causal_dag" & !="causality" & !="causal_links" & !="causal_edges" & !="depends_on" & !="dependencies" & !="cross_system" & !="cross_system_dependency" & !="cross_system_dependencies" & !="system_dependency" & !="system_dependencies" & !="depends_on_system" & !="target_system" & !="source_system" & !="cross_instance" & !="cross_instance_dependency" & !="cross_instance_dependencies" & !="instance_dependency" & !="instance_dependencies" & !="depends_on_instance" & !="target_instance" & !="source_instance"
#ContextVariableName: #FieldName

#TemplateFields: [Name=#TemplateFieldName]:         #TemplateValueExpr
#ContextScope: [Name=#ContextVariableName]:         #ContextVariable
#GlobalContextScope: [Name=#ContextVariableName]:   #GlobalContextVariable
#SystemContextScope: [Name=#ContextVariableName]:   #SystemContextVariable
#InstanceContextScope: [Name=#ContextVariableName]: #InstanceContextVariable
#EventContextScope: [Name=#ContextVariableName]:    #EventContextVariable

#ContextVariable: close({
	source:       #ContextValueSource
	description?: string
})

#GlobalContextVariable: close({
	source:       #GlobalContextValueSource
	description?: string
})

#SystemContextVariable: close({
	source:       #SystemContextValueSource
	description?: string
})

#InstanceContextVariable: close({
	source:       #InstanceContextValueSource
	description?: string
})

#EventContextVariable: close({
	source:       #EventContextValueSource
	description?: string
})

// Closed schema for the exported JSON IR. CUE source files are merged at the
// package root, so top-level closure is enforced by validating the exported JSON
// against this definition in scripts/export-ir.sh.
#IR: close({
	ir_version: "0.2"
	generators?: [ID=#Identifier]: #Generator & {id: ID}

	systems?: [ID=#Identifier]: #System & {
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

	scenario: #Scenario
})

ir_version: "0.2"

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

scenario: #Scenario

// Explicit MVP boundary guards for common drift fields in CUE source. The final
// exported JSON is additionally checked against the closed #IR schema.
entity?:       null & string
attack_chain?: null & string
attack_ratio?: null & string
stage?:        null & string

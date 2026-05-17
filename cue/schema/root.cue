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
	params:    *{} | #JSONObject
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

#TemplateFieldName:   #FieldName & !="shared" & !="share" & !="scope" & !="cache" & !="script" & !="hook" & !="plugin"
#ContextVariableName: #FieldName

#TemplateFields:       [Name=#TemplateFieldName]: #TemplateValueExpr
#ContextScope:         [Name=#ContextVariableName]: #ContextVariable
#GlobalContextScope:   [Name=#ContextVariableName]: #GlobalContextVariable
#SystemContextScope:   [Name=#ContextVariableName]: #SystemContextVariable
#InstanceContextScope: [Name=#ContextVariableName]: #InstanceContextVariable
#EventContextScope:    [Name=#ContextVariableName]: #EventContextVariable

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

	scenario: #Scenario
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

scenario: #Scenario

// Explicit MVP boundary guards for common drift fields in CUE source. The final
// exported JSON is additionally checked against the closed #IR schema.
entity?:       null & string
attack_chain?: null & string
attack_ratio?: null & string
stage?:        null & string

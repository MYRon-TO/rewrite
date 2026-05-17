package rewrite

// Context defines variables and their sharing scope. Runtime cache behavior is
// intentionally outside the schema; the schema only states what variables exist
// and how their values are sourced.

#Context: close({
	global: *{} | #ContextScope
	system: *{} | {
		[SystemID=#Identifier]: #ContextScope
	}
	instance: *{} | {
		[ScopeID=#Identifier]: #ContextScope
	}
	event: *{} | #ContextScope
})

#ContextScope: [Name=#FieldName]: #ContextVariable

#ContextVariable: close({
	source:       #ContextValueSource
	description?: string
})

#ContextValueSource: #ConstValueSource | #GeneratorValueSource | #ContextRefValueSource

#ConstValueSource: close({
	const: #JSON
})

#GeneratorValueSource: close({
	generator: #Identifier
	params?:   #JSONObject
})

#ContextRefValueSource: close({
	ref: #ContextRef
})

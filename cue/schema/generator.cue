package rewrite

// A generator is a declarative value-source contract. It names a built-in
// generator type and its JSON-compatible parameters; it does not expose code,
// hooks or plugin execution.

#Generator: close({
	id: #Identifier

	// Built-in generator type name. The exact built-in set is frozen later by
	// the runtime generator phase.
	type: #Identifier

	params: *{} | #JSONObject
	description?: string
})

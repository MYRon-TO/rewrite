package rewrite

// A generator is a declarative value-source contract. It names a built-in
// generator type and its JSON-compatible parameters; it does not expose code,
// hooks or plugin execution.

#Generator: close({
	id: #Identifier

	// Built-in generator type name. Phase 3 freezes the MVP built-in set.
	type: #BuiltinGeneratorType

	params: *{} | #MVPJSONObject
	description?: string
})

#BuiltinGeneratorType: "fixed" | "const" | "choice" | "number_range" | "range" | "ipv4" | "username" | "simple_username" | "uuid" | "session_id" | "sequence" | "simple_words" | "simple_filename"

package rewrite

// A system is a logging/system namespace boundary. It does not define runtime
// instance counts, scheduling or cross-system causality.

#System: close({
	id: #Identifier

	// Human-facing classification only, for example "linux_auth" or
	// "web_access". It must not encode execution behavior.
	kind?: string

	description?: string
})

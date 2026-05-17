package rewrite

// Context defines variables and their sharing scope. Runtime cache behavior is
// intentionally outside the schema; the schema only states what variables exist
// and how their values are sourced.
//
// Reference direction is lifecycle-safe:
// - global variables may reference only ctx.global.*
// - system variables may reference ctx.global.* or ctx.system.*
// - instance variables may reference ctx.global.*, ctx.system.* or ctx.instance.*
// - event variables may reference any context scope
// This keeps long-lived context from depending on shorter-lived values, which
// would otherwise make runtime caching semantics wobble like a shopping cart
// with one cursed wheel.

#Context: close({
	global: *{} | #GlobalContextScope
	system: *{} | {
		[SystemID=#Identifier]: #SystemContextScope
	}
	instance: *{} | {
		[ScopeID=#Identifier]: #InstanceContextScope
	}
	event: *{} | #EventContextScope
})

package rewrite

// Template defines event fields and field bindings. It does not define sharing
// policy and it must not decide whether an event is an attack.

#Template: close({
	id:     #Identifier
	system: #Identifier

	event_type?:  string
	description?: string

	fields: [Name=#FieldName]: #ValueExpr
})

#ValueExpr: #ConstExpr | #ContextRefExpr | #InterpolationExpr

#ConstExpr: close({
	const: #JSON
})

#ContextRefExpr: close({
	ref: #ContextRef
})

#InterpolationExpr: close({
	interpolate: string
})

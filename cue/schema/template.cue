package rewrite

// Template defines event fields and field bindings. It does not define sharing
// policy and it must not decide whether an event is an attack.
//
// Field bindings are deliberately restricted to three expression shapes:
// - {const: <json>}
// - {ref: "ctx.<scope>.<name>"}
// - {interpolate: "plain text ${ctx.<scope>.<name>}"}
// Unknown expression keys are rejected by closed schemas; reserved field names
// that look like sharing policy or execution hooks are also rejected. Templates
// are field binders, not a tiny script host wearing a fake mustache.

#Template: close({
	id:     #Identifier
	system: #Identifier

	event_type?:  string
	description?: string

	fields: #TemplateFields
})

// Backward-compatible aliases for other schema files and documentation snippets.
#ValueExpr:             #TemplateValueExpr
#ConstExpr:             #TemplateConstExpr
#ContextRefExpr:        #TemplateContextRefExpr
#InterpolationExpr:     #TemplateInterpolationExpr

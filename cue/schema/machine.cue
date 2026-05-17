package rewrite

// A machine is a local behavior graph: states, transitions, terminal markers
// and emitted templates. Cross-machine and cross-system chains are outside MVP.

#Machine: close({
	id:     #Identifier
	system: #Identifier
	start:  #Identifier

	attack: *{enabled: false} | #MachineAttack

	states: [StateID=#Identifier]: #State

	description?: string
})

#MachineAttack: close({
	enabled:      bool | *false
	type?:        string
	technique?:   string
	tactic?:      string
	description?: string
})

#State: close({
	kind: *"benign" | "benign" | "attack"

	attack_label?: #AttackLabel

	emit: *[] | [...#Emit]
	transitions: *[] | [...#Transition]
	terminal: *false | bool

	description?: string
})

#AttackLabel: close({
	type?:      string
	technique?: string
	tactic?:    string
	name?:      string
})

#Emit: close({
	template: #Identifier
	count:    *1 | #EmitCount
})

#EmitCount: (int & >=0) | #EmitCountRange

#EmitCountRange: close({
	min: int & >=0
	max: int & >=min
})

#Transition: close({
	to:     #Identifier
	weight: *1 | (number & >0)
})

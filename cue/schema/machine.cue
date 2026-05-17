package rewrite

// A machine is a local behavior graph: states, transitions, terminal markers
// and emitted templates. Cross-machine and cross-system chains are outside MVP.

#Machine: close({
	id:     #Identifier
	system: #Identifier
	start:  #Identifier

	// Attack metadata is only a default label source. It must not encode an
	// attack flow; the flow remains inside states and transitions.
	attack: #MachineAttack

	states: [StateID=#Identifier]: #State

	// If attack support is disabled, the machine must not contain attack states.
	// This keeps attack modeling local and explicit instead of letting a disabled
	// attack block silently coexist with attack-labeled behavior.
	if attack.enabled == false {
		states: [StateID=#Identifier]: {
			kind: *"benign" | "benign"
		}
	}

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

	// State-level labels are overrides for attack states only. Benign states must
	// not carry attack metadata; otherwise templates and labels start leaking into
	// each other like a badly packed lunchbox.
	attack_label?: #AttackLabel
	if kind != "attack" {
		attack_label?: _|_
	}

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

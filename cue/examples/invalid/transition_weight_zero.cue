package rewrite

ir_version: "0.1"
systems: {s: {}}
machines: {m: {system: "s", start: "a", states: {a: {transitions: [{to: "b", weight: 0}]}, b: {terminal: true}}}}
instance_groups: {g: {machine: "m", count: 1}}
time_profiles: {tp: {start: "2026-01-01T00:00:00Z"}}
scenario: {id: "bad_transition_weight", systems: ["s"], instance_groups: ["g"], time_profile: "tp"}

package rewrite

ir_version: "0.2"
systems: {s: {}}
templates: {t: {system: "s", fields: {message: {const: "x"}}}}
machines: {m: {system: "s", start: "a", states: {a: {emit: [{template: "t", count: {min: 10, max: 1}}], terminal: true}}}}
instance_groups: {g: {machine: "m", count: 1}}
time_profiles: {tp: {start: "2026-01-01T00:00:00Z"}}
scenario: {id: "bad_emit_count", systems: ["s"], instance_groups: ["g"], time_profile: "tp"}

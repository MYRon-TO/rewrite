package rewrite

ir_version: "0.1"
systems: {s: {}}
templates: {t: {system: "s", fields: {message: {interpolate: "run ${exec.rm}"}}}}
machines: {m: {system: "s", start: "a", states: {a: {emit: [{template: "t"}], terminal: true}}}}
instance_groups: {g: {machine: "m", count: 1}}
time_profiles: {tp: {start: "2026-01-01T00:00:00Z"}}
scenario: {id: "bad_interpolation", systems: ["s"], instance_groups: ["g"], time_profile: "tp"}

# CUE 编写指南

## 1. 先记住一个结论

写 `rewrite` 的 CUE 文件，本质上是在声明一份可导出为 JSON IR 的日志仿真场景。

推荐按下面顺序编写：

```text
system
  -> generator
  -> context
  -> template
  -> machine
  -> instance_group
  -> time_profile
  -> scenario
```

其中：

- `system` 定义日志域或系统域。
- `generator` 定义值从哪里来。
- `context` 定义值在哪个作用域共享。
- `template` 定义一条日志长什么样。
- `machine` 定义行为状态机。
- `instance_group` 定义同一个状态机跑多少份实例。
- `time_profile` 定义时间起点、clock skew，以及可选的行为节奏和抖动。
- `scenario` 定义本次仿真的入口。

Rust 运行时不读取 CUE。你写的 CUE 会先通过 `cue export` 导出成 JSON IR，再由 `rewrite validate` 和 `rewrite generate` 处理。别把 CUE 写成执行脚本；CUE 在这里是定义和校验工具，不是披着配置皮的小型运行时。

## 2. 从现有示例开始

第一次写新场景，建议复制一个已有示例：

```bash
cp cue/examples/ssh_bruteforce.cue cue/examples/my_scenario.cue
```

然后按流程校验、导出、验证和生成：

```bash
scripts/validate-cue.sh cue/examples/my_scenario.cue
scripts/export-ir.sh cue/examples/my_scenario.cue
cargo run -- validate --ir generated/examples/my_scenario.json
cargo run -- generate --ir generated/examples/my_scenario.json --out out/my_scenario.jsonl --seed 42
```

如果你想从空文件开始，也可以使用下一节的最小模板。

## 3. 最小可运行 CUE 模板

下面是一个单系统、单状态机、多实例的最小示例。保存为 `cue/examples/minimal_login.cue` 后即可导出和生成。

```cue
package rewrite

ir_version: "0.1"

systems: {
	linux_auth: {
		kind:        "linux_auth"
		description: "Linux authentication log domain"
	}
}

generators: {
	username_pool: {
		type: "choice"
		params: {
			values: ["alice", "bob", "carol"]
		}
	}

	source_ip_pool: {
		type: "ipv4"
		params: {
			cidr: "10.0.0.0/24"
		}
	}

	session_id: {
		type: "uuid"
	}
}

context: {
	global: {
		tenant_id: {
			source: {const: "tenant-demo"}
		}
	}

	system: {
		linux_auth: {
			hostname: {
				source: {const: "ssh-01"}
			}
			program: {
				source: {const: "sshd"}
			}
		}
	}

	instance: {
		ssh_login_once: {
			user: {
				source: {generator: "username_pool"}
			}
			src_ip: {
				source: {generator: "source_ip_pool"}
			}
			session_id: {
				source: {generator: "session_id"}
			}
		}
	}

	event: {}
}

templates: {
	ssh_login_success: {
		system:     "linux_auth"
		event_type: "ssh_login_success"
		fields: {
			tenant_id:  {ref: "ctx.global.tenant_id"}
			host:       {ref: "ctx.system.hostname"}
			program:    {ref: "ctx.system.program"}
			user:       {ref: "ctx.instance.user"}
			src_ip:     {ref: "ctx.instance.src_ip"}
			session_id: {ref: "ctx.instance.session_id"}
			result:     {const: "success"}
			message: {
				interpolate: "Accepted password for ${ctx.instance.user} from ${ctx.instance.src_ip}"
			}
		}
	}
}

machines: {
	ssh_login_once: {
		system: "linux_auth"
		start:  "login_success"

		attack: {
			enabled: false
		}

		states: {
			login_success: {
				kind: "benign"
				emit: [
					{template: "ssh_login_success"},
				]
				terminal: true
			}
		}
	}
}

instance_groups: {
	login_sessions: {
		machine: "ssh_login_once"
		enabled: true
		count:   3
	}
}

time_profiles: {
	default_time: {
		start:                  "2026-05-17T00:00:00Z"
		timezone:               "UTC"
		granularity:            "millisecond"
		monotonic_per_instance: true
		system_clock_skew: {
			linux_auth: {milliseconds: 0}
		}
	}
}

scenario: {
	id:              "minimal_login_demo"
	description:     "Minimal login scenario"
	systems:         ["linux_auth"]
	instance_groups: ["login_sessions"]
	time_profile:    "default_time"
}
```

注意：对象集合使用 map 形式，例如 `systems.linux_auth`、`templates.ssh_login_success`。导出的 JSON IR 中会自动包含内部 `id` 字段，ID 来自 map key。

## 4. 每个对象怎么写

### 4.1 `system`：先划分日志域

`system` 是命名空间边界，用来区分不同日志域。

```cue
systems: {
	linux_auth: {
		kind: "linux_auth"
	}
	web_access: {
		kind: "web_access"
	}
}
```

它不负责实例数量、调度、跨系统依赖。多系统只是并行参与同一个场景，不表示它们之间有因果链。

### 4.2 `generator`：声明值来源

`generator` 只声明值如何生成，不写代码、不写 hook、不调用外部插件。

常用类型包括：

| 类型                           | 用途                                |
| ------------------------------ | ----------------------------------- |
| `choice`                       | 从 `params.values` 中随机选一个值。 |
| `number_range` / `range`       | 从 `[min, max]` 中取数。            |
| `ipv4`                         | 生成 IPv4，常用 `params.cidr`。     |
| `uuid`                         | 生成 UUID 字符串。                  |
| `session_id`                   | 生成 session id。                   |
| `username` / `simple_username` | 生成简单用户名。                    |
| `fixed` / `const`              | 返回固定值。                        |
| `simple_words`                 | 生成简单词组                        |
| `simple_filename`              | 生成简单带后缀的文件名              |

示例：

```cue
generators: {
	user_pool: {
		type: "choice"
		params: {
			values: ["alice", "bob"]
		}
	}

	client_ip: {
		type: "ipv4"
		params: {
			cidr: "192.0.2.0/24"
		}
	}
}
```

### 4.3 `context`：决定值在哪里共享

`context` 有四层作用域：

| 作用域     | 写法                            | 共享范围          | 典型变量                 |
| ---------- | ------------------------------- | ----------------- | ------------------------ |
| `global`   | `context.global`                | 整个 scenario     | tenant、environment      |
| `system`   | `context.system.<system_id>`    | 单个系统          | hostname、service        |
| `instance` | `context.instance.<machine_id>` | 单个 machine 实例 | user、src_ip、session_id |
| `event`    | `context.event`                 | 单条事件          | pid、password_candidate  |

生命周期引用方向必须从长到短，不能反过来：

```text
global   -> 只能引用 global
system   -> 可引用 global/system
instance -> 可引用 global/system/instance
event    -> 可引用 global/system/instance/event
```

正确示例：

```cue
context: {
	global: {
		tenant_id: {source: {const: "tenant-a"}}
	}

	instance: {
		ssh_login_flow: {
			user: {source: {generator: "user_pool"}}
			tenant_copy: {source: {ref: "ctx.global.tenant_id"}}
		}
	}
}
```

错误示例：

```cue
context: {
	global: {
		// 错：global 不能依赖 event。长生命周期变量依赖短生命周期变量，像让房东跟着租客搬家。
		bad: {source: {ref: "ctx.event.pid"}}
	}
}
```

### 4.4 `template`：只定义事件字段

`template` 表示一类日志事件的字段结构。字段绑定只支持三种形式：

```cue
fields: {
	result: {const: "success"}
	user:   {ref: "ctx.instance.user"}
	msg:    {interpolate: "user=${ctx.instance.user} host=${ctx.system.hostname}"}
}
```

规则：

1. `{const: ...}` 用于常量 JSON 值。
2. `{ref: "ctx.<scope>.<name>"}` 用于引用上下文变量。
3. `{interpolate: "...${ctx.<scope>.<name>}..."}` 用于简单字符串插值。

不要在模板里写脚本、循环、条件、攻击判定或共享策略。模板只负责字段绑定，不负责判断这是不是攻击。

### 4.5 `machine`：用状态机描述行为

`machine` 是行为图，包含起始状态、状态集合、状态发射的模板和转移关系。

```cue
machines: {
	ssh_login_flow: {
		system: "linux_auth"
		start:  "start"
		attack: {enabled: false}

		states: {
			start: {
				transitions: [
					{to: "login_success", weight: 1},
				]
			}

			login_success: {
				kind: "benign"
				emit: [{template: "ssh_login_success"}]
				terminal: true
			}
		}
	}
}
```

状态字段重点是：

| 字段                | 含义                                          |
| ------------------- | --------------------------------------------- |
| `kind`              | `benign` 或 `attack`，默认 `benign`。         |
| `emit`              | 当前状态发出哪些模板事件。                    |
| `emit.count`        | 发出多少条同类事件，可为整数或 `{min, max}`。 |
| `transitions`       | 可转移到哪些状态。                            |
| `transition.weight` | 当前状态下的局部分支权重，必须大于 0。        |
| `terminal`          | 是否终止。                                    |

`transition.weight` 不是全局比例控制。它只控制当前状态下候选边的相对权重。把它当最终攻击占比硬约束，属于拿温度计当遥控器。

### 4.6 攻击分支：只能放在单个 `machine` 内

当前版本的攻击建模很收敛：攻击只是某个 `machine` 内的攻击状态和攻击转移分支。

```cue
machines: {
	ssh_login_flow: {
		system: "linux_auth"
		start:  "start"

		attack: {
			enabled:   true
			type:      "ssh_bruteforce"
			technique: "T1110"
			tactic:    "credential_access"
		}

		states: {
			start: {
				transitions: [
					{to: "normal_login", weight: 95},
					{to: "bruteforce", weight: 5},
				]
			}

			normal_login: {
				kind: "benign"
				emit: [{template: "ssh_login_success"}]
				terminal: true
			}

			bruteforce: {
				kind: "attack"
				attack_label: {
					type:      "ssh_bruteforce"
					technique: "T1110"
					name:      "SSH brute-force password guessing"
				}
				emit: [
					{
						template: "ssh_login_failed"
						count: {min: 20, max: 80}
					},
				]
				terminal: true
			}
		}
	}
}
```

规则：

- `machine.attack` 只提供默认攻击标签，不描述攻击流程。
- `state.kind: "attack"` 表示该状态发出的事件会被标记为攻击。
- `state.attack_label` 只应出现在攻击状态上。
- 同一个 `template` 可以被正常状态和攻击状态复用。
- 攻击比例主要由状态机结构和 `transition.weight` 共同决定，不由 `instance_group.attack_ratio` 控制。

当前版本不要写顶层 `attack_chain`、跨系统攻击链、跨实例依赖、攻击阶段 `stage` 或事件级 DAG。把这些塞进第一版，等于往折叠伞上装避雷针，造型很猛，下雨先倒霉。

### 4.7 `instance_group`：批量运行同一个状态机

多实例不要复制多份 `machine`，而是使用 `instance_group.count`。

```cue
instance_groups: {
	login_sessions: {
		machine: "ssh_login_flow"
		enabled: true
		count:   20
	}
}
```

每个实例独立运行同一个 `machine`：

- 共享 `global` 和所属 `system` 上下文。
- 拥有独立的 `instance` 上下文。
- 不支持实例之间通信、触发或依赖。

### 4.8 `time_profile`：时间起点、抖动和行为节奏

基础配置：

```cue
time_profiles: {
	default_time: {
		start:                  "2026-05-17T00:00:00Z"
		timezone:               "UTC"
		granularity:            "millisecond"
		monotonic_per_instance: true
		system_clock_skew: {
			linux_auth: {milliseconds: 0}
			web_access: {milliseconds: 250}
		}
	}
}
```

字段含义：

- `start`：全局起始时间，使用 RFC3339 格式。
- `timezone`：当前建议使用 `UTC`。
- `granularity`：最小推进粒度，支持 `second`、`millisecond`、`microsecond`、`nanosecond`。
- `monotonic_per_instance`：同一实例内时间是否保证不倒退。
- `system_clock_skew`：系统级时钟偏移，支持固定值或范围值。

固定偏移：

```cue
system_clock_skew: {
	linux_auth: {milliseconds: 0}
	web_access: {milliseconds: 250}
}
```

范围偏移：

```cue
system_clock_skew: {
	linux_auth: {min_milliseconds: -50, max_milliseconds: 50}
}
```

#### 4.8.1 `event_time`：行为节奏和抖动

如果不配置 `event_time`，运行时会按 `granularity` 固定 tick 推进。要生成更拟真的日志，可以配置：

```text
下一条事件时间 = 上一条事件时间
  + base_delay_ms
  + behavior_jitter_ms
  + error_jitter_ms
```

`event_time` 使用三层优先级：

```text
states.<machine_id>.<state_id> > event_types.<event_type> > default
```

示例：

```cue
time_profiles: {
	default_time: {
		start:                  "2026-05-17T00:00:00Z"
		timezone:               "UTC"
		granularity:            "millisecond"
		monotonic_per_instance: true
		system_clock_skew: {
			linux_auth: {milliseconds: 0}
		}

		event_time: {
			default: {
				base_delay_ms: {min_milliseconds: 800, max_milliseconds: 2500}
				behavior_jitter_ms: {min_milliseconds: -200, max_milliseconds: 500}
				error_jitter_ms: {min_milliseconds: -20, max_milliseconds: 20}
			}

			event_types: {
				ssh_login_failed: {
					base_delay_ms: {min_milliseconds: 700, max_milliseconds: 2200}
					behavior_jitter_ms: {min_milliseconds: -200, max_milliseconds: 500}
					error_jitter_ms: {min_milliseconds: -20, max_milliseconds: 20}
				}
			}

			states: {
				ssh_login_flow: {
					bruteforce: {
						base_delay_ms: {min_milliseconds: 20, max_milliseconds: 80}
						behavior_jitter_ms: {min_milliseconds: -10, max_milliseconds: 30}
						error_jitter_ms: {min_milliseconds: -2, max_milliseconds: 2}
					}
				}
			}
		}
	}
}
```

这里 `states.ssh_login_flow.bruteforce` 会覆盖 `event_types.ssh_login_failed`，适合模拟高频 burst。慢速扫描也用 state 级覆盖，别靠乱造一堆 `event_type` 硬凑，那个配置会长得像被猫踩过的键盘。

#### 4.8.2 时间延迟字段写法

`base_delay_ms`、`behavior_jitter_ms`、`error_jitter_ms` 都支持两种写法。

固定值：

```cue
base_delay_ms: {milliseconds: 1000}
```

范围值：

```cue
base_delay_ms: {min_milliseconds: 500, max_milliseconds: 1800}
```

建议：

- `default` 写通用节奏。
- `event_types` 写大类行为节奏，例如登录、文件操作、系统监控。
- `states` 只写特殊状态，例如高频 burst、慢速扫描、长时间等待。
- 如果 `behavior_jitter_ms + error_jitter_ms` 导致时间不递增，且 `monotonic_per_instance: true`，运行时会至少推进一个 `granularity` tick。

### 4.9 `scenario`：唯一运行入口

一个可执行 IR 文件必须包含一个 `scenario`。

```cue
scenario: {
	id:              "my_demo"
	description:     "My first rewrite scenario"
	systems:         ["linux_auth"]
	instance_groups: ["login_sessions"]
	time_profile:    "default_time"
}
```

`scenario` 只组合系统、实例组和时间配置。它不负责调度细节，也不负责攻击链编排。

## 5. CUE 复用建议

CUE 的优势是合并、默认值和组合。建议把公共片段抽出来，再在具体对象里覆盖。

示例：复用公共字段组。

```cue
#CommonAuthFields: {
	tenant_id: {ref: "ctx.global.tenant_id"}
	host:      {ref: "ctx.system.hostname"}
	program:   {ref: "ctx.system.program"}
	user:      {ref: "ctx.instance.user"}
}

templates: {
	ssh_login_success: {
		system:     "linux_auth"
		event_type: "ssh_login_success"
		fields: #CommonAuthFields & {
			result: {const: "success"}
		}
	}

	ssh_login_failed: {
		system:     "linux_auth"
		event_type: "ssh_login_failed"
		fields: #CommonAuthFields & {
			result: {const: "failed"}
			src_ip: {ref: "ctx.instance.src_ip"}
		}
	}
}
```

建议：

1. 公共字段用 `#Common...` 定义。
2. 公共上下文片段也可以用 CUE 合并复用。
3. 不要通过复制多份 `machine` 模拟多实例。
4. 不要用 CUE 的复杂表达式伪造运行时逻辑。

## 6. 校验与导出流程

### 6.1 校验 CUE

```bash
scripts/validate-cue.sh cue/examples/my_scenario.cue
```

这一步检查 CUE 结构和 schema 约束，例如：

- 字段名是否合法。
- `transition.weight > 0`。
- `emit.count.min <= emit.count.max`。
- `attack_label` 是否错误地出现在 benign 状态。
- 是否出现被禁止的 `attack_chain`、`entity`、`script` 等字段。

### 6.2 导出 JSON IR

```bash
scripts/export-ir.sh cue/examples/my_scenario.cue
```

默认输出：

```text
generated/examples/my_scenario.json
```

### 6.3 验证 IR

```bash
cargo run -- validate --ir generated/examples/my_scenario.json
```

这一步检查引用关系，例如：

- `scenario.systems` 是否引用已定义的 system。
- `instance_group.machine` 是否存在。
- `machine.start` 是否是有效 state。
- `transition.to` 是否指向有效 state。
- `emit.template` 是否引用已定义 template。

### 6.4 生成日志

```bash
cargo run -- generate \
  --ir generated/examples/my_scenario.json \
  --out out/my_scenario.jsonl \
  --seed 42
```

相同 IR 和相同 `--seed` 会产生可复现输出。

## 7. 常见错误清单

### 7.1 把模板当攻击规则

错误想法：

```text
模板名是 ssh_login_failed，所以一定是攻击。
```

正确做法：

```text
state.kind 决定是否攻击；template 只定义日志字段。
```

同一个 `ssh_login_failed` 模板既可以表示正常用户输错密码，也可以表示爆破尝试。

### 7.2 在 `instance_group` 里写 `attack_ratio`

不要写：

```cue
instance_groups: {
	login_sessions: {
		machine: "ssh_login_flow"
		count: 100
		attack_ratio: 0.05
	}
}
```

当前版本不支持 `attack_ratio`。攻击路径由 `machine.states[].transitions[].weight` 控制。

### 7.3 在模板或 generator 参数里塞脚本

不要写：

```cue
fields: {
	user: {script: "return random_user()"}
}
```

也不要写 hook、plugin、external call。当前版本是声明式定义层，不是脚本引擎。

### 7.4 让长生命周期 context 引用短生命周期 context

不要让 `global` 引用 `event`，也不要让 `system` 引用 `instance`。

```cue
context: {
	system: {
		linux_auth: {
			// 错：system 不能引用 instance。
			bad_user: {source: {ref: "ctx.instance.user"}}
		}
	}
}
```

### 7.5 复制多份 machine 模拟多实例

不要这样：

```text
ssh_login_user_1
ssh_login_user_2
ssh_login_user_3
```

应该这样：

```cue
instance_groups: {
	login_sessions: {
		machine: "ssh_login_flow"
		count: 3
	}
}
```

### 7.6 引入当前版本不支持的对象

当前版本明确不支持：

- `entity`
- `attack_chain`
- `attack_ratio`
- `stage`
- 跨系统依赖
- 跨实例依赖
- 事件级 DAG
- 任意脚本执行

这些不是“暂时没例子”，而是当前 MVP 的边界。硬塞进去，校验失败是正常的。

## 8. 编写新场景的推荐步骤

按这个顺序写，出错概率最低：

1. 定义 `systems`：先确定有哪些日志域。
2. 定义 `generators`：列出需要随机或生成的值。
3. 定义 `context`：决定哪些值全局共享、系统共享、实例共享或事件私有。
4. 定义 `templates`：写每类日志事件的字段。
5. 定义 `machines`：写状态、转移和事件发射。
6. 定义 `instance_groups`：指定每个 machine 跑多少实例。
7. 定义 `time_profiles`：设置时间起点和 clock skew。
8. 定义 `scenario`：把系统、实例组和时间配置组合起来。
9. 运行 `scripts/validate-cue.sh`。
10. 运行 `scripts/export-ir.sh`。
11. 运行 `cargo run -- validate --ir ...`。
12. 运行 `cargo run -- generate --ir ...`。

一句话总结：先把对象边界写清楚，再让状态机产生事件，最后用实例组批量运行。不要一上来就设计跨系统攻击链；那不是第一版能力，是未来版本的坑位。

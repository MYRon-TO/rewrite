# 02. 核心对象模型

## 1. `system`

`system` 表示一个日志域或系统域，是定义的命名空间边界。

### 作用

* 为日志模板和行为模型提供归属域。
* 承载系统级上下文。
* 支持多系统并行建模。

### 示例语义

* `linux_auth`
* `web_access`
* `windows_security`

### 不负责

* 不定义运行实例数量。
* 不定义调度方式。
* 不承担跨系统关系语义。

## 2. `machine`

`machine` 表示一个行为模型，即状态机 / 马尔可夫链定义。

### 作用

* 定义状态集合。
* 定义起始状态。
* 定义状态之间的转移关系。
* 定义状态是否终止。
* 定义状态与事件模板的关联。
* 在当前版本中，允许正常状态和攻击状态共存。

### 建模方式

推荐使用“状态局部声明”的方式定义转移：

* 当前状态是什么。
* 可以转移到哪些状态。
* 各边权重是多少。
* 是否为终止状态。
* 当前状态发出哪些事件。

### 攻击相关语义

攻击建模放在 `machine` 内部，而不是新增顶层攻击链对象。

```text
machine
  states
    benign state
    attack state
  transitions
    normal branch
    attack branch controlled by weight
```

`machine.attack` 只提供默认攻击标签元信息，不描述攻击执行流程，也不承担编排职责。

建议结构：

```cue
attack?: {
  enabled: bool | *false
  type?: string
  technique?: string
  tactic?: string
  description?: string
}
```

字段语义：

* `enabled`：该 `machine` 是否包含攻击分支。
* `type`：攻击类型，例如 `ssh_bruteforce`、`web_scan`。
* `technique`：可选 ATT&CK technique ID，例如 `T1110`。
* `tactic`：可选 ATT&CK tactic，例如 `credential_access`。
* `description`：人类可读说明。

约束：

* 当任一状态的 `kind` 为 `attack` 时，建议 `machine.attack.enabled == true`。
* 当 `machine.attack.enabled == false` 时，不应出现 `kind: "attack"` 的状态。
* `machine.attack` 不描述攻击流程。

### 状态字段

状态建议支持：

```cue
kind?: "benign" | "attack" | *"benign"

attack_label?: {
  type?: string
  technique?: string
  tactic?: string
  name?: string
}

emit?: [...{
  template: string
  count?: int | {
    min: int
    max: int
  } | *1
}]

transitions?: [...{
  to: string
  weight?: number | *1
}]

terminal?: bool | *false
```

字段语义：

* `kind`：该状态发出的事件是否属于攻击行为。
* `attack_label`：该状态特有的攻击标签覆盖。
* `emit.count`：该状态可发出多少条同类事件。
* `transition.weight`：同一状态下候选转移的局部采样权重。

约束：

* `attack_label` 只应出现在 `kind: "attack"` 的状态上。
* `kind` 不改变状态机执行语义，只影响标签和语义解释。
* `emit.count` 不表示脚本循环，只表示声明式事件数量范围。
* `transition.weight > 0`。

### 不负责

* 不负责运行时实例化。
* 不负责随机采样实现。
* 不负责多实例调度。
* 不负责跨 `machine` 攻击链。

## 3. `template`

`template` 表示一类日志事件的结构定义。

### 作用

* 定义事件类型。
* 定义字段集合。
* 定义字段值如何绑定。
* 支持模板复用与覆盖。

### 攻击相关语义

`template` 不知道自己是否攻击。

同一模板可以被正常状态和攻击状态复用。攻击标签由发出模板的 `state.kind` 决定，而不是由模板固化决定。

例如 `ssh_login_failed` 既可能是正常用户输错密码，也可能是爆破。把模板名直接等同攻击，是典型“告警洁癖”：看见失败登录就掏红牌，像保安见到外卖小哥就喊入侵。

### 字段绑定只允许三类形式

1. 常量。
2. 引用上下文变量。
3. 基于已有变量或字段的受限表达式。

### 不负责

* 不负责共享策略。
* 不负责生成器实现。
* 不负责复杂控制流。
* 不固化攻击语义。

## 4. `generator`

`generator` 表示一种值生成契约。

### 作用

* 为变量或字段提供值来源声明。
* 描述该值的语义类别和参数约束。
* 与执行层的生成器实现解耦。

### 示例语义

* 用户名生成器。
* IP 地址生成器。
* 时间戳生成器。
* URL 路径生成器。

### 不负责

* 不暴露具体语言函数。
* 不承载运行时逻辑。
* 不直接定义缓存与作用域。

## 5. `context`

`context` 表示共享变量定义层。

### 作用

* 定义变量名。
* 定义变量值来源。
* 定义变量作用域。
* 定义模板如何引用这些变量。

### 当前版本支持的作用域

* `global`：整个 `scenario` 共享。
* `system`：某个 `system` 内共享。
* `instance`：某个状态机实例内共享。
* `event`：仅当前事件有效。

### 攻击相关语义

攻击状态仍使用现有 `context` 作用域。攻击相关变量，例如 `attacker_ip`、`target_user`，第一版优先放入 `instance` 或 `event` 作用域。

当前版本不引入跨 `machine` 的攻击事实共享。

### 不负责

* 不实现缓存。
* 不实现生命周期管理。
* 不实现跨实例身份映射。

## 6. `scenario`

`scenario` 表示一次仿真的总体场景定义，是定义层中的顶层组合对象。

### 作用

* 声明本次仿真包含哪些系统。
* 声明启用哪些 `instance_group`。
* 定义全局上下文。
* 绑定统一时间配置。

### 攻击相关语义

`scenario` 不控制攻击比例。它只组合启用哪些 `instance_group`。

如果某个 `instance_group` 引用含攻击分支的 `machine`，是否进入攻击路径由该 `machine` 内的 `transition.weight` 决定。

### 不负责

* 不实现执行调度。
* 不实现实例排序。
* 不承载复杂因果关系。
* 不偷偷实现攻击链编排。

## 7. `instance_group`

`instance_group` 表示同一行为模型的批量实例化规则。

### 作用

* 指定使用哪个 `machine`。
* 指定实例数量或分布。
* 指定实例局部参数。
* 指定实例之间是否互相独立。

### 攻击相关语义

`instance_group` 不提供 `attack_ratio`。

批量实例化时，每个实例独立按照 `machine.transition.weight` 采样。多实例下实际攻击实例数是随机结果，不是硬性配额。

别在这里再造一套比例控制。两个方向盘开一辆车，最后多半是开进沟里。

### 不负责

* 不负责跨实例关联。
* 不负责实例间消息交互。
* 不定义复杂依赖关系。
* 不定义攻击配额。

## 8. `time_profile`

`time_profile` 表示定义层中的时间约束配置。

### 作用

* 定义本次场景的起始时间。
* 定义时间粒度。
* 定义是否要求单实例时间单调递增。
* 定义系统级时钟偏移配置。

### 当前版本支持

* 全局起始时间。
* 单实例时间单调。
* 系统级 clock skew 配置。

### 当前版本不支持

* 复杂因果时间约束语言。
* 跨系统事件依赖窗口建模。

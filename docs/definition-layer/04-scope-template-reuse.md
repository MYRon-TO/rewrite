# 04. 作用域、模板绑定与复用

## 1. `instance` 的定义

`instance` 是当前版本中的基础运行建模单元。

一个 `instance` 表示某个 `machine` 的一次具体运行实体。

它回答：

* 这条行为序列是谁在跑。
* 这条序列当前处于哪个状态。
* 这条序列内部哪些变量应当稳定共享。

## 2. 为什么只保留 `instance`

当前版本不引入 `entity`，原因是：

1. 高频场景主要是“单 actor、单主线”的独立行为序列。
2. `entity` 会增加概念数量和边界复杂度。
3. 跨实例身份稳定性不是第一版重点。

当前版本采用简化策略：

> 将局部稳定对象统一视为 `instance` 内变量，而不额外建立 `entity` 身份层。

如果后续需要支持同一用户跨多个实例复用身份、同一主机跨多个系统复用身份、跨实例 / 跨系统稳定对象图，再引入 `entity`。

## 3. 变量作用域模型

当前版本统一采用四层作用域：

### 3.1 `global`

整个场景共享。

适合定义：

* 全局起始时间。
* 租户 ID。
* 场景 ID。
* 全局背景参数。

### 3.2 `system`

某个系统内共享。

适合定义：

* 主机名。
* 日志源配置。
* 域名。
* 系统级 clock skew。

### 3.3 `instance`

某个状态机实例内共享。

适合定义：

* 用户名。
* session_id。
* src_ip。
* process_root。
* request_id。
* 攻击状态之间需要保持一致的 `attacker_ip`。
* 攻击状态之间需要保持一致的 `target_user`。

### 3.4 `event`

单条事件私有。

适合定义：

* 当前时间戳。
* 单条日志特有随机字段。
* 一次性 nonce。
* 单次尝试中的随机目标字段。

## 4. 攻击变量作用域

攻击状态仍然使用现有 `context` 作用域。

第一版不引入跨 `machine` 的攻击事实共享。因此：

* 同一个攻击路径内要复用的攻击 IP，放入 `ctx.instance.attacker_ip`。
* 每条爆破日志不同的尝试密码，放入 `ctx.event.password_candidate`。
* 攻击状态之间要复用的目标用户，放入 `ctx.instance.target_user`。

不要为了一个 `attacker_ip` 就发明 `fact_store`、`entity_graph` 或别的怪兽。模型还没学会走路，别急着给它装火箭背包。

## 5. 模板绑定规则

模板字段绑定必须保持受限，当前版本建议只支持以下形式。

### 5.1 常量

```cue
result: "success"
```

### 5.2 引用上下文变量

```cue
user: ctx.instance.user
host: ctx.system.hostname
tenant: ctx.global.tenantID
```

### 5.3 受限表达式

```cue
email: "\(ctx.instance.user)@corp.local"
```

### 5.4 明确禁止

当前版本不允许模板中直接出现：

* 任意代码片段。
* 复杂脚本逻辑。
* 外部调用入口。
* 复杂循环与条件流程。
* 攻击判定逻辑。

模板不是脚本宿主，也不是小型 SIEM。

## 6. 模板与攻击标签

模板不固化攻击语义。

正确方式：

```text
state.kind -> label.is_attack
state.attack_label / machine.attack -> label.attack_type, technique, tactic
template -> event fields
```

错误方式：

```text
template.name contains "failed" -> label.is_attack = true
```

失败登录不一定是攻击。正常用户忘密码也很常见，虽然这行为像把脑子缓存清空了，但它不是攻击。

## 7. 复用机制要求

为了避免定义层退化成复制粘贴地狱，当前版本必须支持复用能力。

### 7.1 命名引用

所有核心对象都必须可命名、可引用。

### 7.2 继承 / 覆盖

模板、上下文和配置应支持基于已有定义做少量覆盖。

### 7.3 片段组合

公共字段组、公共上下文组应支持组合复用。

### 7.4 批量实例化

`instance_group` 是复用行为模型的核心机制，禁止通过复制多份 `machine` 定义来模拟多实例。

## 8. CUE 在本项目中的定位

### 8.1 选择 CUE 的原因

CUE 适合作为定义层载体，因为它具备：

* 结构约束。
* 类型校验。
* 默认值。
* 合并与覆盖。
* 组合表达能力。

### 8.2 CUE 的职责

在本项目中，CUE 用于：

* 承载定义层对象。
* 实现结构化组合。
* 提供静态校验。
* 输出规范化中间表示。

### 8.3 CUE 不应承担

CUE 不应承担：

* 执行期随机逻辑。
* 扰动逻辑。
* Hook 逻辑。
* 因果图求解。
* 任意脚本执行。

一句话：CUE 是定义和校验工具，不是小号执行器。

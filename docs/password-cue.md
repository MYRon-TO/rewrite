要求：为日志密钥泄露检测系统编写 CUE，生成用于测试的仿真日志。

### 原始日志格式

原始日志的基本格式为：

```text
<time>, <entity>, <content>
```

例如：

```text
2025-11-19T04:00:00Z, user-host-01, 用户 purplecat0 请求登录系统
2025-11-19T04:00:00.845635600Z, user-host-01, 验证用户 purplecat0 的凭据
```

这里描述的是原始日志内容，不约束 rewrite 项目的最终输出格式。具体建模方式参考：

- `cue/examples/user_system_activity.cue`
- `cue/examples/user_system_activity/*`

### 简化行为路径

与 `examples/user_system_activity` 样例相比，本 CUE 不需要复杂的行为模式或多种攻击手段，只需包含简单的用户行为，以及若干不同类型的密钥泄露场景。

真实密钥泄露视为攻击，必须由 `kind: "attack"` 的状态发出并生成攻击标签。正常日志、占位符日志和易误判日志由 `kind: "benign"` 的状态发出。

### 密钥范围

不要求覆盖项目支持的全部密钥类型，只需包含：

- 人工密码；
- 3～4 种具有代表性的机器密钥。

不得使用真实凭据。机器密钥仅需满足对应类型的外形特征，不要求具备密码学有效性，也不要求能被真实服务接受。

### 泄露类型

不考虑在日志模板中硬编码密钥的异常情况。密钥通过模板变量槽位或复合变量进入日志。

#### 变量密钥泄露

`variable_secret_exposure`：密钥通过模板变量直接出现在日志中。

模板及渲染结果示例：

```text
密码 <*> 验证错误
密码 123456 验证错误
```

其中 `<*>` 仅用于说明模板中的变量槽位，不是最终日志里的脱敏占位符。

#### 嵌套密钥泄露

`nested_secret_exposure`：密钥包含在 JSON、错误详情或其他复合变量中，并随该变量写入日志。

模板及渲染结果示例：

```text
用户尝试登录失败，详细信息：<*>
用户尝试登录失败，详细信息："{'user': 'alice', 'message': '密码 123456 验证错误'}"
```

### 攻击标签

所有真实密钥泄露统一使用：

```cue
attack: {
    enabled: true
    type: "secret_exposure"
    technique: "T1552"
    tactic: "credential_access"
}
```

不同泄露类型通过攻击状态的 `attack_label.name` 区分，同时保持 `type: "secret_exposure"` 不变：

- `variable_secret_exposure`
- `nested_secret_exposure`

生成器和模板本身不判断攻击。是否属于密钥泄露由发出日志的 machine state 决定。

### 占位符日志

最终日志中可以出现以下占位符或脱敏值：

```text
<your-password-here>
<api-key>
******
REDACTED
```

这类内容不包含真实仿真密钥，不计为攻击，不编写 `attack_label`，并由 `kind: "benign"` 的状态发出。

### 易误判日志

CUE 应包含容易被密钥泄露检测系统误判、但实际没有泄露密钥的正常日志，至少覆盖以下类型：

1. 含有“密码”“密钥”“token”“secret”等敏感关键词，但不包含密钥；
2. 使用请求 ID、Trace ID、文件摘要或构建 ID 等常见高熵字符串作为变量；
3. 使用时间日期、UUID、软件版本号、Git commit 或镜像 digest 等外形近似密钥的普通值。

这些日志必须由 `kind: "benign"` 的状态发出，不编写 `attack_label`。

### 行为分支与比例

常规行为日志不包含密钥。当行为涉及密码或机器密钥时，可以通过状态转移进入：

- 正常日志分支；
- 易误判日志分支；
- 真实密钥泄露分支。

期望生成结果中：

- 真实密钥泄露日志约占 20%；
- 易误判日志约占 20%；
- 其余约 60% 为普通正常日志，其中可以包含占位符日志。

通过 `transition.weight` 近似控制上述比例即可，不要求最终数据集严格满足比例。`transition.weight` 表示局部分支概率，实际比例仍会受到状态结构和每个状态发出日志数量的影响。


### 用户行为路径与日志模板

行为路径保持简单：用户发起登录后，系统验证密码；登录成功后，用户执行一次普通操作，随后可能调用外部接口或云服务。每个关键步骤可以进入普通正常、易误判或密钥泄露分支，最后结束本次行为。

本场景使用以下凭据：

- 人工密码：`<user-password>`；
- API Key：`<api-key>`；
- Bearer Token：`<bearer-token>`；
- 云访问密钥对：`<access_key_id>` 和 `<secret_access_key>`。

文档中的 `<var-name>` 表示对应语义的变量槽位，而不是最终日志中的文字。

#### 1. 登录开始

用户首先发起登录并进入密码验证。这一阶段只产生普通正常日志：

```text
用户 <username> 请求登录
开始验证用户 <username> 的密码
```

#### 2. 密码验证

密码验证后进入以下分支之一。

普通正常日志：

```text
用户 <username> 登录成功
用户 <username> 密码错误
用户 <username> 取消登录
```

占位符或脱敏日志，属于正常日志：

```text
用户 <username> 登录失败，密码=<your-password-here>
用户 <username> 登录失败，密码=******
```

易误判日志，包含敏感关键词但没有密钥：

```text
用户 <username> 密码验证失败
用户 <username> 的密码已过期
登录失败，密钥字段为空
```

变量密钥泄露日志，攻击名称为 `variable_secret_exposure`：

```text
用户 <username> 登录失败，密码=<user-password>
```

嵌套密钥泄露日志，攻击名称为 `nested_secret_exposure`：

```text
登录失败，详情={"用户":"<username>","密码":"<user-password>","结果":"拒绝"}
```

登录失败或取消后可以直接结束；登录成功后继续执行普通操作。

#### 3. 普通操作

登录成功后，用户查看数据或提交任务：

```text
用户 <username> 查看数据
用户 <username> 提交任务 <task-id>
任务 <task-id> 执行完成
```

这一阶段加入不含密钥的高熵值和近似密钥外形，用于构造易误判日志：

```text
请求 <request-id> 处理完成
链路 <trace-id> 调用成功
构建版本 <version> 已发布
提交 <git-commit> 已部署
任务时间 <event-time> 已记录
```

以上日志均属于正常日志。

#### 4. 外部接口调用

用户操作可能触发外部接口调用。

普通正常日志：

```text
用户 <username> 调用外部接口
外部接口调用成功
```

占位符日志，属于正常日志：

```text
外部接口配置 api_key=<api-key-here>
请求头 Authorization=Bearer REDACTED
```

易误判日志：

```text
API 密钥检查通过
Token 缓存已清理
请求 <request-id> 鉴权成功
```

变量密钥泄露日志，攻击名称为 `variable_secret_exposure`：

```text
外部接口调用失败，api_key=<api-key>
认证失败，Authorization=Bearer <bearer-token>
```

嵌套密钥泄露日志，攻击名称为 `nested_secret_exposure`：

```text
接口调用失败，详情={"接口":"外部服务","api_key":"<api-key>","请求":"<request-id>"}
认证失败，详情={"类型":"Bearer","token":"<bearer-token>","结果":"拒绝"}
```

#### 5. 云服务调用

用户操作也可能触发云服务调用。

普通正常日志：

```text
用户 <username> 访问云服务
云服务调用成功
```

占位符或脱敏日志，属于正常日志：

```text
云服务配置 access_key=<access-key-here>
云服务配置 secret=******
```

易误判日志：

```text
云密钥轮换完成
Secret 配置检查通过
镜像 <image-digest> 已同步
资源 <resource-uuid> 创建成功
```

变量密钥泄露日志，攻击名称为 `variable_secret_exposure`：

```text
云服务认证失败，access_key_id=<access_key_id>
云服务认证失败，secret_access_key=<secret_access_key>
```

嵌套密钥泄露日志，攻击名称为 `nested_secret_exposure`：

```text
云服务失败，凭据={"access_key_id":"<access_key_id>","secret_access_key":"<secret_access_key>"}
```

#### 6. 行为结束

各分支完成后，以简单的结束日志收尾：

```text
用户 <username> 操作结束
会话 <session-id> 已关闭
```

整体路径可以概括为：

```text
登录开始
  -> 密码验证
     -> 登录失败或取消 -> 结束
     -> 登录成功 -> 普通操作
        -> 直接结束
        -> 外部接口调用 -> 正常、易误判或泄露 -> 结束
        -> 云服务调用 -> 正常、易误判或泄露 -> 结束
```

分支权重应尽量使真实密钥泄露日志约占 20%、易误判日志约占 20%，其余为普通正常日志和占位符日志。无需为了精确比例增加额外状态或复杂循环。

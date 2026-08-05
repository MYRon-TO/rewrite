# 密码与密钥生成器设计

## 1. 目标

为通用仿真日志提供更复杂、拟真的密码和机器密钥值，同时保持定义可校验、运行时离线和结果可复现。

本设计提供：

1. `machine_secret`：生成符合常见格式与长度特征的仿真机器密钥。
2. `corpus_choice`：从内置或外部密码语料中抽样人工密码。
3. 复用现有 `choice`：生成少量弱密码、默认密码和定向值。

## 2. 非目标

本设计不负责：

- 生成可用于生产环境的安全密钥。
- 验证密码或密钥是否真实、有效或曾被泄露。
- 生成密码学有效的 JWT 签名、私钥或云凭据。
- 在运行时下载语料库。
- 模拟完整的人类密码创建行为。
- 新增字段级标注、ground-truth sidecar 或输出协议。
- 修改现有攻击标签架构。

## 3. 设计原则

- 生成与复用分离：生成器定义值来源，`context` 决定生命周期。
- 数据与逻辑分离：人工密码来自语料，机器密钥按格式生成。
- 默认可复现：相同 IR、语料和 seed 产生相同结果。
- 语义解耦：生成器只生成值，不判断该值是否表示攻击。
- 离线运行：外部资源仅允许本地文件。
- 向后兼容：未使用新类型的现有场景和输出保持不变。

## 4. 总体结构

```text
choice -----------┐
corpus_choice ----+-> context -> template -> machine
machine_secret ---┘
```

现有 `Generator.params`、JSON 值、context 四层作用域和模板字段引用继续复用。新增能力仅扩展生成器类型、参数校验和本地资源加载。

## 5. `corpus_choice`

### 5.1 配置

使用内置语料：

```cue
human_password: {
    type: "corpus_choice"
    params: {
        source: {builtin: "human-passwords-v1"}
        min_length: 4
        max_length: 128
        sampling: "uniform"
        deduplicate: true
    }
}
```

使用外部语料：

```cue
human_password: {
    type: "corpus_choice"
    params: {
        source: {path: "passwords/custom.txt"}
        min_length: 6
        max_length: 64
        sampling: "shuffle_cycle"
    }
}
```

### 5.2 数据源

`source` 必须且只能指定一种形式：

- `builtin`：使用随程序发布且带版本号的内置语料。
- `path`：读取本地 UTF-8 文本，每行一个值。

外部相对路径按以下顺序解析：

1. CLI `--asset-dir`；
2. 未指定时使用 IR 文件所在目录。

禁止网络 URL 和运行时下载。

### 5.3 行处理

- 仅移除行尾 LF 或 CRLF，不对整行执行 `trim()`。
- 默认跳过空行，可通过参数显式保留。
- `#` 不具有注释语义。
- 长度按 Unicode 字符数计算。
- 可选去重，默认启用。
- 过滤后语料为空时生成失败。
- 文件大小、行数和单行长度必须设置运行时上限。

### 5.4 抽样

第一版支持：

- `uniform`：有放回均匀抽样。
- `shuffle_cycle`：确定性洗牌后依次取值，一轮结束后重新洗牌。

语料在首次使用时加载，并按规范化数据源缓存。相同 seed、语料内容和调用顺序必须产生相同结果。

### 5.5 内置语料

内置语料控制在 1,000 至 10,000 条，并满足：

- 来源和许可证明确；
- 固定版本并记录处理方式；
- 不包含用户名、邮箱或其他身份组合数据；
- 覆盖多种长度、字符集、语言和常见人工密码形态。

大型或来源受限的语料由用户通过外部文件提供，不提交到仓库。

## 6. `machine_secret`

### 6.1 配置

```cue
api_key: {
    type: "machine_secret"
    params: {
        profile: "api_key"
        prefix: "sim_sk_"
        body_length: 32
        encoding: "base64url"
    }
}
```

### 6.2 Profile

第一版覆盖：

| Profile | 输出 |
| --- | --- |
| `hex` | 指定字节数的十六进制字符串 |
| `base64` | 指定字节数的 Base64 字符串 |
| `base64url` | 指定字节数的无填充 Base64URL 字符串 |
| `uuid` | UUID v4 外形字符串 |
| `api_key` | 可配置前缀和主体长度的 API Key |
| `bearer_token` | Bearer Token 外形字符串 |
| `jwt_shape` | 三段式 JWT/JWS 外形字符串 |
| `pem_shape` | 带 PEM 边界和换行的 Base64 外形文本 |
| `cloud_access_key` | 云访问密钥 ID 外形字符串 |
| `cloud_secret_pair` | 包含 ID 和 secret 的 JSON 对象 |
| `connection_string` | 含凭据的连接字符串 |
| `private_token` | Git、CI 或包仓库 Token 外形字符串 |

`hex`、`base64` 和 `base64url` 使用 `byte_length` 表示原始字节数；前缀型 profile 使用 `body_length` 表示主体字符数。不提供含义不明确的统一 `length` 参数。

推荐覆盖 16、24、32、48、64 字节，以及常见的 20、32、40、64 字符主体长度。

### 6.3 仿真边界

- JWT header 和 payload 必须可进行 Base64URL 解码，但 signature 不保证可验证。
- PEM 主体必须符合 Base64 和换行格式，但不保证是有效 DER 或私钥。
- 云凭据不保证可被真实服务接受。
- 默认使用 `sim_` 或等价标记；需要精确外形时必须显式关闭标记。

除 `base64` 外，不引入 JWT、RSA、Ed25519、HMAC 或哈希依赖。

## 7. 弱密码

少量弱密码、默认密码和定向值继续使用现有 `choice`：

```cue
weak_password: {
    type: "choice"
    params: {
        values: ["123456", "password", "admin", "qwerty"]
    }
}
```

本设计不新增 `weak_password` 类型。

## 8. 作用域

推荐用法：

| 值 | 作用域 |
| --- | --- |
| 系统服务密钥 | `system` |
| 单账户 API Key 或密码 | `instance` |
| 临时 Token、JWT、单次输入密码 | `event` |
| 场景统一测试值 | `global` |

生成器不实现生命周期缓存；复用仍由现有 context 管理。

## 9. 攻击标签

仿真密码或密钥被意外写入日志时，使用现有 machine 攻击标签表达，不新增生成器级标签或字段级标注。

推荐标签：

```cue
attack: {
    enabled: true
    type: "secret_exposure"
    technique: "T1552"
    tactic: "credential_access"
}
```

对应状态使用：

```cue
kind: "attack"
```

标签语义：

- `type: "secret_exposure"` 表示密码、Token、API Key 或私钥材料暴露在日志中。
- `technique: "T1552"` 对应 ATT&CK `Unsecured Credentials`，作为最接近的通用映射。
- `tactic: "credential_access"` 对应 Credential Access。

T1552 描述攻击者查找未受保护凭据，并不专门定义“应用意外记录密钥”。因此该 technique 是近似分类，项目内的准确业务语义以 `secret_exposure` 为准。

生成器不得根据所生成值自动设置攻击标签。同一个生成器可以同时被正常状态和攻击状态复用，是否属于密钥暴露由发出日志的 machine state 决定。

## 10. 校验与限制

CUE 和 Rust 验证共同覆盖：

- 未知生成器类型或 profile；
- 互斥数据源同时出现或均未出现；
- 非法长度、空范围和超出资源上限；
- 不支持的编码；
- 不存在或不可读的外部文件；
- 非法 UTF-8；
- 过滤后语料为空。

CUE 负责结构和静态参数约束，Rust 负责文件、语料内容和运行时参数校验。

## 11. 兼容性

新增生成器类型属于 IR 合约扩展，使用 IR `0.2`。运行时继续读取现有 `0.1`，但 `0.1` 不允许使用 `machine_secret` 和 `corpus_choice`。

本设计不修改事件输出结构、模板结构、攻击标签结构或现有 context 语义。

## 12. 依赖

必需新增：

```toml
base64 = "0.23"
```

第一版不支持压缩语料，不新增解压依赖。外部语料版本和内容由使用者管理，内置语料随程序版本固定。

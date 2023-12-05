# 配置文件说明
运行 dash-node 时需要提前配置配置文件所在文件夹。当未指定时，默认为 dash-node 可执行文件同目录下的 config 文件夹。示例目录结构及说明如下：

```
config
├── config.yaml // 主配置文件
├── peers // 包含所有对等节点（包括自己）的配置文件，文件命名随意
│   ├── 0.yaml
│   ├── 1.yaml
│   ├── 2.yaml
│   ├── 3.yaml
│   └── ...
└── sec_key // 节点的 ed25519 私钥，pem 格式，可通过工具生成
```

## 主配置文件说明
主配置文件需要包含以下内容:
```
# dash-node peer 共识监听地址及端口
peer_listen_address: 127.0.0.1:8080
# dash-node 与 client 通信监听地址及端口
client_listen_address: 127.0.0.1:8081
# 视图超时，单位毫秒：当前视图超时前的等待时间
minimum_view_timeout_ms: 500
# 同步时，单个响应中请求同步对等方发送块数量限制
sync_request_limit: 10
# 同步响应超时时间，单位毫秒
sync_response_timeout_ms: 5000
```

## 对等节点配置文件说明
说明如下
```
# 对等节点的地址和端口
host_addr: 127.0.0.1:8080
# 对等节点的公钥
public_key: db3MWGjrGbXuxXyLCU02rh/MyowpwfHIh8etJF5wVmI=
```

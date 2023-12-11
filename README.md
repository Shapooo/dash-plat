# dash-plat

包含四个子仓库：
- dash-node：共识节点
- dash-client：客户端，发送交易和性能统计
- dash-common：存放公共代码
- dash-network: 公用的网络设施
- dash-tools：一些辅助小工具

另外有 scripts 文件夹，用于存放一些方便的脚本。

## 使用方法
本地基础测试：
1. 运行 scripts/basic_local_test.sh 脚本
2. 进入项目根目录生成的 experiment 文件夹，此时已经编译并配置好四个共识节点运行所需文件
3. 分别运行四个子文件夹内的 dash-node 及 dash-client 程序
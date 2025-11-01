# Phase 7 Roadmap - 统计和限流系统

**创建时间**: 2025-10-31
**状态**: 📋 规划完成
**预期时间**: 1-2 周

## 核心目标

1. **统计服务完善** - 完整的使用统计和成本追踪
2. **速率限制实现** - 真实的限流执行和并发控制
3. **实时指标收集** - 多维度统计和监控
4. **OpenAI 转发完善** - 完成转发服务实现

## 实施计划

### Week 1: 统计和限流核心
- **Day 1-2**: 成本计算集成
  - 集成 PricingService 到所有路由
  - 替换 `cost: 0.0` 为真实计算
  - 测试成本准确性

- **Day 3-4**: 速率限制执行
  - 在中间件启用限流检查
  - 实现成本限制
  - 添加 429 响应格式

- **Day 5**: 多维度统计
  - 创建 UsageStatsService
  - 实现统计聚合

### Week 2: OpenAI 完善和优化
- **Day 1-2**: OpenAI 转发实现
  - 完成 Responses 转发逻辑
  - 实现流式响应支持
  - 解除 3 个 ignored 测试

- **Day 3**: 实时指标收集
  - 创建 MetricsService
  - 实现指标聚合

- **Day 4-5**: 优化和测试
  - 并发控制优化
  - 性能基准测试
  - 文档更新

## 关键任务

### Phase 7.1: 统计服务完善
1. **成本计算集成** (P0) - 1 天
   - 文件: `src/routes/*.rs`
   - 替换所有 `cost: 0.0`
   - 集成 PricingService

2. **多维度统计** (P1) - 1 天
   - 文件: 新建 `src/services/usage_stats.rs`
   - 按时间/模型/用户统计
   - Redis 数据结构设计

3. **实时指标收集** (P2) - 1 天
   - 文件: 新建 `src/services/metrics.rs`
   - 滑动窗口统计
   - P50/P90/P99 延迟

### Phase 7.2: 速率限制实现
1. **速率限制执行** (P0) - 0.5 天
   - 文件: `src/middleware/auth.rs`
   - 启用 `check_rate_limit()`
   - 返回 429 状态码

2. **成本限制实现** (P1) - 0.5 天
   - 文件: `src/services/api_key.rs`
   - 实现 `check_cost_limit()`
   - 成本超限拒绝

3. **并发控制优化** (P2) - 1 天
   - 文件: `src/services/account_scheduler.rs`
   - 基于 TTL 自动过期
   - 并发监控

### Phase 7.3: OpenAI 转发完善
1. **Responses 转发逻辑** (P1) - 1 天
   - 文件: `src/services/openai_relay.rs`, `src/routes/openai.rs`
   - 请求格式转换
   - 响应格式转换
   - 错误处理

2. **流式响应支持** (P2) - 1 天
   - SSE 流转发
   - Usage 数据捕获
   - 流式测试

3. **解除 ignored 测试** (P1) - 0.5 天
   - 解除 3 个 ignored
   - 确保所有测试通过

## 预期成果

### 代码增量
- 新增文件: 2-3 个
- 修改文件: 10+ 个
- 新增代码: ~1,500 行
- 新增测试: ~500 行

### 测试增量
- 新增单元测试: 15+
- 新增集成测试: 20+
- 总测试数: 280 → 315+

### 功能增强
- ✅ 真实成本计算和追踪
- ✅ 生产级速率限制
- ✅ 多维度使用统计
- ✅ 实时监控指标
- ✅ OpenAI 完整支持

## 成功标准

### 功能完整性
- ✅ 所有路由集成真实成本计算
- ✅ 速率限制真实执行并返回 429
- ✅ 成本限制真实执行
- ✅ 多维度统计数据可查询
- ✅ OpenAI 转发逻辑完整
- ✅ 所有 ignored 测试解除

### 性能指标
- ✅ 成本计算延迟 < 1ms
- ✅ 速率限制检查延迟 < 5ms
- ✅ 统计查询延迟 < 10ms
- ✅ 并发请求处理 > 10,000/s

## 风险和缓解

1. **成本计算性能影响**
   - 缓解: 异步成本记录
   - 备选: LRU 缓存定价数据

2. **速率限制 Redis 负载**
   - 缓解: Redis 管道批量操作
   - 备选: 本地缓存 + Redis 同步

3. **OpenAI 转发复杂度**
   - 缓解: 降低优先级
   - 备选: 可放到 Phase 8

## 立即行动

### Day 1 任务
1. 成本计算集成
   - 修改 `src/routes/api.rs::handle_messages`
   - 替换 `cost: 0.0`
   - 验证成本正确性

2. 速率限制执行
   - 修改 `src/middleware/auth.rs::authenticate_api_key`
   - 添加 `check_rate_limit()` 调用
   - 添加 429 错误响应

3. 编写测试
   - 成本计算集成测试
   - 速率限制集成测试

## 参考文档

- `PHASE7_ROADMAP.md` - 完整路线图
- `src/services/pricing_service.rs` - 定价服务
- `src/services/api_key.rs` - API Key 服务
- `tests/pricing_service_integration_test.rs` - 定价测试

## 进度追踪

- Phase 1-5: ✅ 完成
- Phase 6: ✅ 完成 (40% 总进度)
- **Phase 7**: 📋 规划完成，准备开始
- Phase 8-13: ⏳ 待规划

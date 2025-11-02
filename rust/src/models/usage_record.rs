/// 使用记录数据结构
#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub key_id: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub cost: f64,
}

impl UsageRecord {
    /// 创建新的使用记录
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key_id: String,
        model: String,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_read_tokens: i64,
        cost: f64,
    ) -> Self {
        Self {
            key_id,
            model,
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            cost,
        }
    }
}

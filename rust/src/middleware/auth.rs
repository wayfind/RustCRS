use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::models::api_key::ApiKey;
use crate::services::{AdminService, ApiKeyService, Claims};
use crate::utils::error::AppError;

/// API Key 认证状态
///
/// 存储在请求扩展中,供后续处理器使用
#[derive(Clone, Debug)]
pub struct AuthState {
    /// 已验证的 API Key
    pub api_key: ApiKey,
}

/// API Key 认证中间件
///
/// 从 Authorization header 提取 API Key,验证其有效性,
/// 并将认证状态存储到请求扩展中
///
/// # 工作流程
///
/// 1. 提取 Authorization header
/// 2. 解析 Bearer token
/// 3. 验证 API Key
/// 4. 检查权限(可选,由路由处理器完成)
/// 5. 将 API Key 信息存储到请求扩展
///
/// # 错误处理
///
/// - 缺少 Authorization header: 401 Unauthorized
/// - 格式错误: 401 Unauthorized
/// - API Key 无效: 401 Unauthorized
/// - API Key 已禁用: 401 Unauthorized
/// - API Key 已过期: 401 Unauthorized
pub async fn authenticate_api_key(
    State(service): State<Arc<ApiKeyService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. 提取 Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    // 2. 解析 Bearer token
    let api_key = parse_bearer_token(auth_header)?;

    // 3. 验证 API Key
    let validated_key = service.validate_key(&api_key).await?;

    // 4. 检查速率限制
    service.check_rate_limit(&validated_key).await?;

    // 5. 存储认证状态到请求扩展
    let auth_state = AuthState {
        api_key: validated_key,
    };
    request.extensions_mut().insert(auth_state);

    // 6. 继续处理请求
    Ok(next.run(request).await)
}

/// 解析 Bearer token
///
/// 从 Authorization header 中提取 API Key
///
/// # 参数
///
/// * `auth_header` - Authorization header 值
///
/// # 返回
///
/// 提取的 API Key
///
/// # 格式
///
/// 支持以下格式:
/// - `Bearer <api_key>`
/// - `<api_key>` (直接提供)
fn parse_bearer_token(auth_header: &str) -> Result<String, AppError> {
    if auth_header.starts_with("Bearer ") {
        // 标准 Bearer token 格式
        Ok(auth_header
            .strip_prefix("Bearer ")
            .unwrap_or("")
            .trim()
            .to_string())
    } else {
        // 直接提供 API Key (兼容模式)
        Ok(auth_header.trim().to_string())
    }
}

/// 可选认证中间件
///
/// 如果存在 Authorization header 则验证,否则允许匿名访问
///
/// 用于某些需要区分认证用户和匿名用户的端点,
/// 但不强制要求认证
pub async fn optional_authenticate_api_key(
    State(service): State<Arc<ApiKeyService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 检查是否有 Authorization header
    if let Some(auth_header) = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        // 如果有,则尝试验证
        match parse_bearer_token(auth_header) {
            Ok(api_key) => match service.validate_key(&api_key).await {
                Ok(validated_key) => {
                    let auth_state = AuthState {
                        api_key: validated_key,
                    };
                    request.extensions_mut().insert(auth_state);
                }
                Err(_) => {
                    // 验证失败,但不阻止请求 (可选认证)
                    // 路由处理器可以检查是否存在 AuthState
                }
            },
            Err(_) => {
                // 解析失败,但不阻止请求
            }
        }
    }

    // 继续处理请求
    Ok(next.run(request).await)
}

/// 从请求扩展中提取认证状态
///
/// # 参数
///
/// * `request` - HTTP 请求
///
/// # 返回
///
/// 如果存在,返回 Some(AuthState),否则返回 None
pub fn extract_auth_state(request: &Request) -> Option<AuthState> {
    request.extensions().get::<AuthState>().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bearer_token_with_prefix() {
        let header = "Bearer cr_test123456";
        let result = parse_bearer_token(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "cr_test123456");
    }

    #[test]
    fn test_parse_bearer_token_without_prefix() {
        let header = "cr_test123456";
        let result = parse_bearer_token(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "cr_test123456");
    }

    #[test]
    fn test_parse_bearer_token_with_extra_spaces() {
        let header = "Bearer   cr_test123456   ";
        let result = parse_bearer_token(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "cr_test123456");
    }

    #[test]
    fn test_parse_bearer_token_empty() {
        let header = "Bearer ";
        let result = parse_bearer_token(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }
}

/// JWT 认证状态
///
/// 存储在请求扩展中,供后续处理器使用
#[derive(Clone, Debug)]
pub struct JwtAuthState {
    /// 已验证的 JWT Claims
    pub claims: Claims,
}

/// JWT 认证中间件
///
/// 从 Authorization header 提取 JWT token,验证其有效性,
/// 并将认证状态存储到请求扩展中
///
/// # 工作流程
///
/// 1. 提取 Authorization header
/// 2. 解析 Bearer token
/// 3. 验证 JWT token
/// 4. 将 Claims 存储到请求扩展
///
/// # 错误处理
///
/// - 缺少 Authorization header: 401 Unauthorized
/// - 格式错误: 401 Unauthorized
/// - Token 无效: 401 Unauthorized
/// - Token 已过期: 401 Unauthorized
pub async fn authenticate_jwt(
    State(service): State<Arc<AdminService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. 提取 Authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

    // 2. 解析 Bearer token
    let token = parse_bearer_token(auth_header)?;

    // 3. 验证 JWT token
    let claims = service.verify_token(&token)?;

    // 4. 存储认证状态到请求扩展
    let jwt_state = JwtAuthState { claims };
    request.extensions_mut().insert(jwt_state);

    // 5. 继续处理请求
    Ok(next.run(request).await)
}

/// 从请求扩展中提取 JWT 认证状态
///
/// # 参数
///
/// * `request` - HTTP 请求
///
/// # 返回
///
/// 如果存在,返回 Some(JwtAuthState),否则返回 None
pub fn extract_jwt_state(request: &Request) -> Option<JwtAuthState> {
    request.extensions().get::<JwtAuthState>().cloned()
}

/// 验证管理员角色
///
/// 检查 JWT claims 中的 role 字段是否为 "admin"
///
/// # 参数
///
/// * `request` - HTTP 请求
///
/// # 返回
///
/// 如果是管理员角色返回 Ok(()),否则返回 Forbidden 错误
pub fn require_admin_role(request: &Request) -> Result<(), AppError> {
    let jwt_state = extract_jwt_state(request)
        .ok_or_else(|| AppError::Unauthorized("Missing JWT authentication".to_string()))?;

    if jwt_state.claims.role != "admin" {
        return Err(AppError::Forbidden("Admin role required".to_string()));
    }

    Ok(())
}

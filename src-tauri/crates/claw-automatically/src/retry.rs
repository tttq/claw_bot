use crate::error::{AutomaticallyError, Result};
use crate::types::RetryPolicy;
use std::future::Future;

const DEFAULT_RETRY_POLICY: RetryPolicy = RetryPolicy {
    max_attempts: 3,
    retry_delay_ms: 1000,
    backoff_multiplier: 1.5,
};

pub async fn with_retry<F, Fut, T>(
    operation_name: &str,
    policy: Option<&RetryPolicy>,
    f: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let policy = policy.unwrap_or(&DEFAULT_RETRY_POLICY);
    let mut delay_ms = policy.retry_delay_ms;
    let mut last_error: Option<AutomaticallyError> = None;

    for attempt in 0..policy.max_attempts {
        match f().await {
            Ok(result) => {
                if attempt > 0 {
                    log::info!(
                        "[Retry:{}] Succeeded on attempt {}/{}",
                        operation_name,
                        attempt + 1,
                        policy.max_attempts
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt + 1 < policy.max_attempts {
                    log::warn!(
                        "[Retry:{}] Attempt {}/{} failed, retrying in {}ms: {}",
                        operation_name,
                        attempt + 1,
                        policy.max_attempts,
                        delay_ms,
                        last_error.as_ref().unwrap()
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    delay_ms = (delay_ms as f64 * policy.backoff_multiplier) as u64;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        AutomaticallyError::Automation(format!(
            "{} failed after {} attempts",
            operation_name, policy.max_attempts
        ))
    }))
}

pub async fn with_timeout<F, Fut, T>(
    operation_name: &str,
    timeout_ms: u64,
    f: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    match tokio::time::timeout(tokio::time::Duration::from_millis(timeout_ms), f()).await {
        Ok(result) => result,
        Err(_elapsed) => Err(AutomaticallyError::Timeout(format!(
            "Operation '{}' timed out after {}ms",
            operation_name, timeout_ms
        ))),
    }
}

pub async fn with_retry_and_timeout<F, Fut, T>(
    operation_name: &str,
    policy: Option<&RetryPolicy>,
    timeout_ms: u64,
    f: F,
) -> Result<T>
where
    F: Fn() -> Fut + Copy,
    Fut: Future<Output = Result<T>>,
{
    with_retry(
        operation_name,
        policy,
        move || with_timeout(operation_name, timeout_ms, f),
    )
    .await
}

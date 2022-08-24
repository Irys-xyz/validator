use std::marker::PhantomData;

use chrono::Duration;

use futures::Future;

pub trait Runtime {
    type Sleep: Future<Output = ()> + Send;
    fn sleep(duration: Duration) -> Self::Sleep;
}

impl Runtime for tokio::runtime::Handle {
    type Sleep = tokio::time::Sleep;
    fn sleep(duration: Duration) -> Self::Sleep {
        tokio::time::sleep(duration.to_std().unwrap())
    }
}

impl Runtime for actix_rt::Runtime {
    type Sleep = actix_rt::time::Sleep;
    fn sleep(duration: Duration) -> Self::Sleep {
        actix_rt::time::sleep(duration.to_std().unwrap())
    }
}

pub enum RetryBackoffStrategy {
    Constant(Duration),
    // Exponential: duration * 2^iteration, maybe we should add some random delay
    Exponential(Duration),
}

impl Default for RetryBackoffStrategy {
    fn default() -> Self {
        RetryBackoffStrategy::Exponential(Duration::seconds(1))
    }
}

pub enum RetryControl<T> {
    /// indicate the operation succeeded
    Success(T),
    /// indicate the operation failed
    Fail(T),
    /// Retry after delay
    ///
    /// If maximum number of retries is reached, returns the passed value
    /// If `None`, use backoff strategy for calculating the delay
    Retry(T, Option<Duration>),
}

pub struct RetryBuilder<Runtime, R, T> {
    max_retries: u8,
    backoff: RetryBackoffStrategy,
    success_handler: fn(value: T) -> R,
    failure_handler: fn(value: T, max_retries_reached: bool) -> R,
    phantom: PhantomData<Runtime>,
}

impl<Runtime, R, T> RetryBuilder<Runtime, R, T> {
    pub fn max_retries(self, value: u8) -> RetryBuilder<Runtime, R, T> {
        RetryBuilder {
            max_retries: value,
            ..self
        }
    }

    pub fn backoff(self, value: RetryBackoffStrategy) -> RetryBuilder<Runtime, R, T> {
        RetryBuilder {
            backoff: value,
            ..self
        }
    }

    pub fn success_handler(self, cb: fn(final_value: T) -> R) -> RetryBuilder<Runtime, R, T> {
        RetryBuilder {
            success_handler: cb,
            failure_handler: self.failure_handler,
            max_retries: self.max_retries,
            backoff: self.backoff,
            phantom: self.phantom,
        }
    }

    pub fn failure_handler(
        self,
        cb: fn(final_value: T, max_retries_reached: bool) -> R,
    ) -> RetryBuilder<Runtime, R, T> {
        RetryBuilder {
            failure_handler: cb,
            success_handler: self.success_handler,
            max_retries: self.max_retries,
            backoff: self.backoff,
            phantom: self.phantom,
        }
    }
}

impl<Runtime, R, T> RetryBuilder<Runtime, R, T>
where
    Runtime: self::Runtime,
{
    pub async fn run_with_context<'a, 'b, Ctx, Fut, F>(self, ctx: &'a Ctx, payload: F) -> R
    where
        'a: 'b,
        Fut: Future<Output = RetryControl<T>>,
        F: Fn(&'b Ctx) -> Fut + 'b,
    {
        let mut final_value: Option<T> = None;
        for i in 0..self.max_retries {
            match payload(ctx).await {
                RetryControl::Success(value) => return (self.success_handler)(value),
                RetryControl::Fail(value) => return (self.failure_handler)(value, false),
                RetryControl::Retry(value, Some(duration)) => {
                    final_value = Some(value);
                    Runtime::sleep(duration).await;
                    continue;
                }
                RetryControl::Retry(value, None) => {
                    final_value = Some(value);
                    match self.backoff {
                        RetryBackoffStrategy::Constant(duration) => {
                            Runtime::sleep(duration).await;
                        }
                        RetryBackoffStrategy::Exponential(base) => {
                            // TODO: we should probably add some random time here
                            Runtime::sleep(Duration::seconds(
                                base.num_seconds()
                                    .saturating_mul(2u8.pow(i.into()).into())
                                    .into(),
                            ))
                            .await
                        }
                    }
                    continue;
                }
            }
        }

        return (self.failure_handler)(final_value.take().unwrap(), true);
    }

    pub async fn run<Fut, F>(self, payload: F) -> R
    where
        Fut: Future<Output = RetryControl<T>>,
        F: Fn() -> Fut,
    {
        let mut final_value: Option<T> = None;
        for i in 0..self.max_retries {
            match payload().await {
                RetryControl::Success(value) => return (self.success_handler)(value),
                RetryControl::Fail(value) => return (self.failure_handler)(value, false),
                RetryControl::Retry(value, Some(duration)) => {
                    final_value = Some(value);
                    Runtime::sleep(duration).await;
                    continue;
                }
                RetryControl::Retry(value, None) => {
                    final_value = Some(value);
                    match self.backoff {
                        RetryBackoffStrategy::Constant(duration) => {
                            Runtime::sleep(duration).await;
                        }
                        RetryBackoffStrategy::Exponential(base) => {
                            // TODO: we should probably add some random time here
                            Runtime::sleep(Duration::seconds(
                                base.num_seconds()
                                    .saturating_mul(2u8.pow(i.into()).into())
                                    .into(),
                            ))
                            .await
                        }
                    }
                    continue;
                }
            }
        }
        return (self.failure_handler)(final_value.take().unwrap(), true);
    }
}

pub fn retry<Runtime, T>() -> RetryBuilder<Runtime, T, T>
where
    Runtime: self::Runtime,
{
    RetryBuilder {
        max_retries: 3,
        backoff: RetryBackoffStrategy::default(),
        success_handler: |value| value,
        failure_handler: |value, _| value,
        phantom: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        pin::Pin,
        sync::atomic::{AtomicI8, Ordering},
    };

    use chrono::Duration;
    use futures::{executor::LocalPool, Future};

    use super::{retry, RetryControl, Runtime};

    struct Client;

    impl Client {
        async fn call(&self, fail: bool) -> Result<(), String> {
            if !fail {
                Ok(())
            } else {
                Err("Request failed".into())
            }
        }
    }

    pub struct MockSleep;

    impl Future for MockSleep {
        type Output = ();

        fn poll(
            self: Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Self::Output> {
            std::task::Poll::Ready(())
        }
    }

    impl Runtime for LocalPool {
        type Sleep = MockSleep;
        // This is good enough implementation for testing, we don't
        // really want to wait for the whole duration in unit tests.
        fn sleep(_: Duration) -> Self::Sleep {
            MockSleep
        }
    }

    #[test]
    fn retry_returns_correct_value_after_third_try_is_successful() {
        let mut rt = LocalPool::new();

        let ctx = (Client, AtomicI8::new(0));
        let res = {
            rt.run_until(retry::<LocalPool, _>().max_retries(5).run_with_context(
                &ctx,
                |(client, counter)| async move {
                    match client
                        .call(counter.fetch_add(1, Ordering::Relaxed) < 2)
                        .await
                    {
                        ret @ Ok(_) => RetryControl::Success(ret),
                        ret @ Err(_) => RetryControl::Retry(ret, None),
                    }
                },
            ))
        };

        assert_eq!(res, Ok(()));
    }

    #[test]
    fn retry_returs_error_provided_by_the_callback_if_max_retries_is_reached() {
        let mut rt = LocalPool::new();

        let res: Result<(), String> = {
            rt.run_until(
                retry::<LocalPool, _>()
                    .max_retries(5)
                    .run(|| async { RetryControl::Retry(Err("Failed".to_string()), None) }),
            )
        };

        assert_eq!(res, Err("Failed".to_string()));
    }
}

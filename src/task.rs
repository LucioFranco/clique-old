use {
    futures::{
        compat::TokioDefaultSpawner,
        future::FutureExt,
        future::RemoteHandle,
        task::{SpawnError, SpawnExt},
    },
    std::future::Future,
};

pub(crate) fn spawn<F>(f: F)
where
    F: Future + Send + 'static,
{
    TokioDefaultSpawner.spawn(f.map(|_| ())).unwrap();
}

pub(crate) fn spawn_with_handle<F>(f: F) -> Result<RemoteHandle<<F as Future>::Output>, SpawnError>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    TokioDefaultSpawner.spawn_with_handle(f)
}
